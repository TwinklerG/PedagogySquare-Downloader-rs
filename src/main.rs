use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

// Get Hex md5 encoded password
fn hex_md5_stringify(raw_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut md5_encoder = md5::Context::new();
    md5_encoder.write_all(raw_str.as_bytes())?;
    md5_encoder.flush().unwrap();
    Ok(md5_encoder
        .compute()
        .0
        .iter()
        .fold(String::new(), |acc, x| format!("{acc}{:02x}", x)))
}

// Deal with illegal characters of  windows filename
fn filename_filter(mut name: String) -> String {
    let illegal_str = r#"/\:*?”"<>|"#;
    for char in illegal_str.chars() {
        name = name.replace(char, " ");
    }
    name
}

// Some metadata
macro_rules! login_url {
    () => {
        "https://teaching.applysquare.com/Api/User/ajaxLogin"
    };
}
macro_rules! attachment_url_fmt {
    () => {"https://teaching.applysquare.com/Api/CourseAttachment/getList/token/{}?parent_id={}&page={}&plan_id=-1&uid={}&cid={}"};
}
macro_rules! course_info_url_fmt {
    () => {"https://teaching.applysquare.com/Api/Public/getIndexCourseList/token/{}?type=1&usertype=1&uid={}"};
}
macro_rules! attachment_detail_url_fmt {
    () => {"https://teaching.applysquare.com/Api/CourseAttachment/ajaxGetInfo/token/{}?id={}&uid={}&cid={}"};
}

#[derive(Debug, Clone)]
struct Attachment {
    parent_dir: String,
    info: AttachmentList,
}
#[derive(Deserialize, Debug)]
struct AttachmentInfo {
    message: AttachmentMessage,
}
#[derive(Deserialize, Debug)]
struct AttachmentMessage {
    count: usize,
    list: Vec<AttachmentList>,
}
#[derive(Deserialize, Debug, Clone)]
struct AttachmentList {
    id: String,
    title: String,
    ext: String,
    can_download: String,
    size: String,
    path: String,
}

async fn construct_attachment_vec(
    client: &Client,
    token: &str,
    pid: i32,
    uid: &str,
    cid: &str,
    parent_dir: &str,
) -> Vec<Attachment> {
    let mut attachment_vec = Vec::new();

    let attachment_info_url = format!(attachment_url_fmt!(), token, pid, 1, uid, cid);
    let resp = client
        .get(attachment_info_url)
        .send()
        .await
        .unwrap()
        .json::<AttachmentInfo>()
        .await
        .unwrap();

    let file_num = resp.message.count;
    let mut current_page = 1;
    while attachment_vec.len() < file_num {
        let current_url = format!(attachment_url_fmt!(), token, pid, current_page, uid, cid);
        let resp = client
            .get(current_url)
            .send()
            .await
            .unwrap()
            .json::<AttachmentInfo>()
            .await
            .unwrap();
        for attachment in resp.message.list {
            attachment_vec.push(Attachment {
                info: attachment,
                parent_dir: parent_dir.to_string(),
            });
        }
        current_page += 1;
    }
    attachment_vec
}

#[derive(Deserialize, Debug)]
struct Config {
    username: String,
    password: String,
    ext_expel_list: Vec<String>,
    cid_include_list: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load config.json
    let config_file = File::open("config.json").unwrap();
    let config: Config = serde_json::from_reader(config_file)?;

    // Initialize client
    let client = Client::new();
    println!("Trying to log in, please wait ...");
    let mut body = HashMap::new();
    body.insert("email".to_string(), config.username.clone());
    body.insert("password".to_string(), hex_md5_stringify(&config.password)?);
    let req = client.post(login_url!()).form(&[
        ("email", config.username.as_str()),
        ("password", &hex_md5_stringify(&config.password)?),
    ]);

    // Log in
    #[derive(Deserialize)]
    struct LogInfo {
        message: LogMessage,
    }
    #[derive(Deserialize)]
    struct LogMessage {
        uid: String,
        token: String,
    }
    let uid;
    let token;
    match req.send().await.unwrap().json::<LogInfo>().await {
        Ok(log_info) => {
            (uid, token) = (log_info.message.uid, log_info.message.token);
        }
        Err(e) => {
            eprintln!("Login Failed, please check your username & password");
            eprintln!("Login info received: {e}");
            panic!();
        }
    }
    println!("Login Successfully");

    // Get courses infos
    #[derive(Deserialize, Debug)]
    struct CourseInfo {
        message: Vec<CourseMessage>,
    }
    #[derive(Deserialize, Debug)]
    struct CourseMessage {
        cid: String,
        name: String,
    }
    let course_info_url = format!(course_info_url_fmt!(), token, uid);
    let course_infos = client
        .get(course_info_url)
        .send()
        .await
        .unwrap()
        .json::<CourseInfo>()
        .await
        .unwrap();
    let mut cid2name_dict = HashMap::new();
    for course_message in &course_infos.message {
        cid2name_dict.insert(course_message.cid.clone(), course_message.name.clone());
    }
    println!("\nReady to download the following courses:");
    for (cid, name) in &cid2name_dict {
        if config.cid_include_list.is_empty() || config.cid_include_list.contains(cid) {
            println!("Course: {:8}, CID: {:6}", name, cid);
        }
    }

    let cid_list: Vec<_> = cid2name_dict.keys().map(|s| s.to_string()).collect();
    for cid in cid_list {
        let mut tasks = Vec::new();
        if !config.cid_include_list.is_empty() && !config.cid_include_list.contains(&cid) {
            continue;
        }
        let course_name = match cid2name_dict.get(&cid) {
            Some(name) => name.clone(),
            None => {
                println!(
                    "Can't find course name for cid {}, maybe it's a legacy course?",
                    cid
                );
                format!("CID_{}", cid)
            }
        };
        println!("\nDownloading files of course {}", course_name);

        // create dir for this course
        let root_path = std::env::current_dir()
            .unwrap()
            .join("downloads")
            .join(course_name);

        if !Path::exists(&root_path) {
            fs::create_dir_all(&root_path).unwrap();
        }

        // Construct attachment list, with some dirs in it
        let mut course_attachment_list =
            construct_attachment_vec(&client, &token, 0, &uid, &cid, ".").await;

        // Iteratively add files in dirs to global attachment list
        let mut dir_counter = 0;
        for entry in course_attachment_list
            .iter()
            .map(|x| (*x).clone())
            .collect::<Vec<_>>()
        {
            if entry.info.ext == "dir" {
                dir_counter += 1;
                // add dir content to attachment list
                let dir_id = &entry.info.id[..];
                let dir_name = filename_filter(entry.info.title.clone());
                let parent_dir = &entry.parent_dir;
                if !Path::exists(&root_path.join(parent_dir).join(&dir_name)) {
                    fs::create_dir(&*root_path.join(parent_dir).join(&dir_name)).unwrap();
                }

                course_attachment_list.extend(
                    construct_attachment_vec(
                        &client,
                        &token,
                        dir_id.parse::<i32>().unwrap(),
                        &uid,
                        &cid,
                        parent_dir,
                    )
                    .await,
                );
            }
        }
        println!(
            "Get {} files with {} dirs",
            course_attachment_list.len() - dir_counter,
            dir_counter
        );

        let multi_progresses = MultiProgress::new();

        // Download attachments
        for mut entry in course_attachment_list {
            let ext = entry.info.ext.clone();
            if ext == "dir" || config.ext_expel_list.contains(&ext) {
                continue;
            }
            let client2 = client.clone();
            let filename = if entry.info.title.contains(&ext) {
                filename_filter(entry.info.title.clone())
            } else {
                filename_filter(format!("{}.{}", entry.info.title, entry.info.ext))
            };
            let file_path = root_path.join(&entry.parent_dir).join(&filename);
            let filesize = entry.info.size.clone();
            let multi_progresses = multi_progresses.clone();
            let mut flag = true;
            // Get download url for un-downloadable files
            if entry.info.can_download == "0" {
                let attachment_detail_url = format!(
                    attachment_detail_url_fmt!(),
                    &token, &entry.info.id, &uid, &cid
                );
                #[derive(Deserialize)]
                struct AttachDetail {
                    message: DetailMessage,
                }
                #[derive(Deserialize)]
                struct DetailMessage {
                    path: String,
                }
                let resp = client2
                    .get(attachment_detail_url)
                    .send()
                    .await
                    .unwrap()
                    .json::<AttachDetail>()
                    .await
                    .unwrap();

                entry.info.path = resp.message.path;
            }

            let mut resp = client2.get(entry.info.path).send().await.unwrap();
            let content_size = resp.headers()["Content-Length"].to_owned();

            if file_path.exists() && file_path.is_file() {
                // If file is up to date, continue; else, delete and re-download
                if fs::read(&file_path).unwrap().len().to_string() == content_size {
                    println!("File {} is up-to-date", filename);
                    flag = false;
                } else {
                    // println!("Updating File {}", filename);
                    fs::remove_file(&file_path).unwrap();
                }
            }
            if flag {
                tasks.push(tokio::task::spawn(async move {
                    // println!("Downloading {}, filesize = {}", filename, filesize);
                    let mut f = File::create(&file_path).unwrap();
                    // println!("{:?} {}", content_size, content_size.len());
                    let progress_bar = multi_progresses.add(ProgressBar::new(
                        u64::from_str(content_size.to_str().unwrap()).unwrap(),
                    ));
                    progress_bar.set_style(
                        ProgressStyle::with_template(
                            "[{elapsed_precise}] {bar:20.cyan/blue} {pos:>7}/{len:7} {msg}",
                        )
                        .unwrap()
                        .progress_chars("##-"),
                    );
                    progress_bar
                        .set_message(format!("Downloading {}, filesize = {}", filename, filesize));
                    while let Some(chunk) = resp.chunk().await.unwrap() {
                        progress_bar.inc(chunk.len() as u64);
                        f.write_all(&chunk).unwrap();
                    }
                    progress_bar.finish();
                    // multi_progresses.remove(&progress_bar);
                    // println!("Finish download {}", filename);
                }));
                while tasks.len() > 5 {
                    let task = tasks.remove(0);
                    task.await?;
                }
            }
        }
        for task in tasks {
            task.await?;
        }
    }

    Ok(())
}
