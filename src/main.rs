use lazy_static::lazy_static;
use regex::Regex;
use reqwest;
use std::process::Command;
use warp::{
     http::Uri,
     Filter,
     path::FullPath
};

fn extract_store_path(input: &str) -> Option<&str> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"StorePath: ?(?P<hash>.*)").unwrap();
    }
    RE.captures(input)
        .and_then(|cap| cap.name("hash").map(|hash| hash.as_str()))
}

fn is_narinfo(input: &str) -> bool {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[a-zA-Z0-9]{32}.narinfo").unwrap();
    }
    return RE.is_match(input)
}

pub async fn get_store_path(path: String) -> Result<impl warp::Reply, warp::Rejection> {
    match is_narinfo(&path) {
        true => { 
            let res = reqwest::get(format!("https://cache.nixos.org{}", path))
                .await
                .unwrap();
            let body = res.text().await.unwrap();
            let store_path = extract_store_path(&body).unwrap();
            Command::new("nixFlakes")
                .arg("--experimental-features")
                .arg("nix-command")
                .arg("copy")
                .arg("--to")
                .arg("file:/srv/cache")
                .arg(store_path)
                .spawn()
                .expect("The nix command failed to start");
        }
        _ => {}
    }
   Ok(warp::redirect::see_other(format!("https://cache.nixos.org{}", path).parse::<Uri>().unwrap()))
}

fn redirect_and_download() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path::full().and_then(|path: FullPath| get_store_path(path.as_str().to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let route = warp::fs::dir("/srv/cache")
        .or(redirect_and_download());
    warp::serve(route).run(([127, 0, 0, 1], 3030)).await;
    Ok(())
}