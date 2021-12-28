use std::env;
use std::error::Error;
use std::fmt::{Debug};

use regex::Regex;
use serde::Deserialize;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};

const ORIGIN: &str = "https://lanzoux.com";

const USER_AGENTS: (&str, &str) = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.114 Safari/537.36",
    "Mozilla/5.0 (Linux; Android 8.0; Pixel 2 Build/OPD3.170816.012) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.114 Mobile Safari/537.36"
);

enum ClientType {
    PC,
    MOBILE,
}

#[derive(Clone)]
struct FileMeta {
    id: String,
    pwd: String,
}

#[derive(Deserialize, Debug)]
struct FakeResponse {
    dom: String,
    url: String,
}

type Response = Result<String, Box<dyn Error>>;

macro_rules! gen_headers {
    ($client_type:expr) => {
       {
           let mut headers = HeaderMap::new();
            headers.insert(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8".parse().unwrap());
            headers.insert(reqwest::header::REFERER, ORIGIN.parse().unwrap());
            headers.insert(reqwest::header::USER_AGENT, match $client_type {
                ClientType::PC => USER_AGENTS.0,
                ClientType::MOBILE => USER_AGENTS.1,
            }.parse().unwrap());
            headers
       }
    };
}

macro_rules! find_first_match {
    ($text:expr, $pattern:expr) => {
        Regex::new($pattern).unwrap().captures($text).unwrap()[1].to_owned()
    };
}

macro_rules! trim_quote {
    ($str:expr) => {
        $str.trim_matches(&['\'', '\''] as &[_]).parse().unwrap()
    };
}

macro_rules! get_text {
    ($client:expr, $url:expr) => {
        $client.get($url)
            .send()
            .await?
            .text()
            .await?
    };
}

macro_rules! parse_params {
    ($text:expr) => {
        {
            let data = find_first_match!($text.as_str(), r"[^/]{2,}data : \{(.+)}");
            let pairs = data.split(",").map(|pair| {
                let mut split = pair.split(":")
                    .map(|str| str.trim().to_owned())
                    .collect::<Vec<String>>();

                if split[1].starts_with("'") {
                    split[1] = trim_quote!(split[1]);
                } else if split[1].parse::<i32>().is_err() {
                    split[1] = find_first_match!($text.as_str(),
                        format!(r"[^/].*var {}[ ]*=[ ]*'(.+)'", split[1]).as_str());
                };
                split[0] = trim_quote!(split[0]);

                (split[0].to_owned(), split[1].to_owned())
            }).collect::<Vec<(String, String)>>();

            let mut serializer = url::form_urlencoded::Serializer::new(String::new());
            for pair in pairs {
                serializer.append_pair(pair.0.as_str(), pair.1.as_str());
            }

            serializer.finish()
        }
    };
}

async fn get_fake_url(params: String, client: reqwest::Client) -> Response {
    let resp = client.post(format!("{}/ajaxm.php", ORIGIN))
        .header(CONTENT_TYPE, HeaderValue::from_static("application/x-www-form-urlencoded"))
        .body(params)
        .send()
        .await?
        .json::<FakeResponse>()
        .await?;

    Ok(format!("{}/file/{}", resp.dom, resp.url))
}


async fn parse_fake_url_from_pc_page(file: FileMeta) -> Response {
    let client = reqwest::Client::builder()
        .default_headers(gen_headers!(ClientType::PC))
        .build()?;

    let text = get_text!(client,format!("{}/{}", ORIGIN, file.id));
    let params = if !file.pwd.is_empty() {
        format!("{}{}", find_first_match!(text.as_str(), r"[^/]{2,}data : '(.+)'\+pwd"), file.pwd)
    } else {
        let url = format!("{}{}", ORIGIN, find_first_match!(text.as_str(), r#"src="(.{20,})" frameborder"#));
        let text = get_text!(client, url);
        parse_params!(text)
    };

    get_fake_url(params, client).await
}

async fn parse_fake_url_from_mobile_page(file: FileMeta) -> Response {
    let client = reqwest::Client::builder()
        .default_headers(gen_headers!(ClientType::MOBILE))
        .build()?;

    let mut id = file.id.to_owned();
    if !file.id.starts_with("i") {
        let text = get_text!(client, format!("{}/{}", ORIGIN, file.id));
        id = find_first_match!(text.as_str(), r"[^/]{2,}.+ = 'tp/(.+)'");
    }

    let mut text = get_text!(client, format!("{}/tp/{}", ORIGIN, id));
    if file.pwd.is_empty() {
        let path: String = find_first_match!(text.as_str(), r"[^/]{2,}.+'(http[\w\-/:.]{10,})'");
        let params: String = find_first_match!(text.as_str(), r"[^/]{2,}.+'(\?[\w/+=]{20,})'");
        Ok(format!("{}{}", path, params))
    } else {
        text = format!("{};var pwd='{}'", text, file.pwd);
        get_fake_url(parse_params!(text), client).await
    }
}

async fn parse(file: FileMeta, client_type: ClientType) -> Response {
    let fake_url = match client_type {
        ClientType::PC => parse_fake_url_from_pc_page(file).await?,
        ClientType::MOBILE => parse_fake_url_from_mobile_page(file).await?,
    };

    let client = reqwest::Client::builder()
        .default_headers(gen_headers!(client_type))
        .redirect(reqwest::redirect::Policy::custom(|a| a.stop()))
        .build()?;

    Ok(client.head(fake_url)
        .send()
        .await?
        .headers()
        .get(reqwest::header::LOCATION).unwrap().to_str()?.to_owned())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print!("Lack of arguments, examples:\n\t{}\n\t{}",
                 "parse https://lanzoui.com/iRujgdfrkza",
                 "parse https://lanzoui.com/i7tit9c 6svq");
        return Ok(());
    } else if args.len() < 3 {
        args.push("".to_owned());
    }

    args[1] = args[1].split("/").collect::<Vec<&str>>().pop().unwrap().to_owned();
    let file = FileMeta { id: args[1].to_owned(), pwd: args[2].to_owned() };
    for c in [ClientType::MOBILE, ClientType::PC] {
        let resp = parse(file.clone(), c).await;
        if resp.is_ok() {
            print!("{}", resp.unwrap());
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parser_integration_test() {
        let mut futures = vec![];
        [
            ("i7tit9c", "6svq"),
            ("i4wk2oh", ""),
            ("iRujgdfrkza", ""),
            ("dkbdv7", ""),
        ].iter().map(|f| (f.0.to_owned(), f.1.to_owned()))
            .map(|(id, pwd)| FileMeta { id, pwd })
            .for_each(|f| [ClientType::PC, ClientType::MOBILE].into_iter()
                .for_each(|c| futures.push(parse(f.clone(), c))));

        for f in futures {
            assert!(f.await.unwrap().starts_with("https://"));
        }
    }
}