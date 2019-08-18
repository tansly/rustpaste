use actix_web::{post, web, App, HttpResponse, HttpServer};
use futures::Future;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::iter;

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let config = web::Data::new(config);

    HttpServer::new(move || {
        App::new()
            .register_data(config.clone())
            .service(new_paste)
            .route("/{filename}", web::get().to_async(send_paste))
    })
    .bind("127.0.0.1:8080")
    .unwrap()
    .run()?;

    Ok(())
}

#[derive(Clone)]
pub struct Config {
    pub paste_dir: String,
    pub url_base: String,
    // TODO: Fields for HTTP auth (user/pass)
}

impl Config {
    pub fn new() -> Result<Config, &'static str> {
        // TODO: Get values from a config file
        // TODO: Parse command line arguments
        let paste_dir = String::from("./pastes");
        let url_base = String::from("https://localhost");
        Ok(Config {
            paste_dir,
            url_base,
        })
    }
}

#[derive(Deserialize)]
struct Paste {
    pub data: String,
}

// TODO: Consider using multipart formdata instead of urlencoded.
// XXX: This will hang in an infinite loop if the paste directory does not exist.
// We'll probably make sure it exists while parsing config, not here.
#[post("/")]
fn new_paste(
    config: web::Data<Config>,
    paste: web::Form<Paste>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    web::block(move || {
        let mut rng = thread_rng();

        // Paste IDs (= paste file names) are 8 character alphanumeric strings.
        // Here we generate a random ID that is not already in use,
        // and create (and open) a paste file with that ID as its name.
        let (mut file, paste_id) = loop {
            let id: String = iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(8)
                .collect();
            let full_path = format!("{}/{}", config.paste_dir, id);
            if let Ok(file) = OpenOptions::new().write(true).create(true).open(full_path) {
                break (file, id);
            }
        };

        let paste_url = format!("{}/{}", config.url_base, paste_id);
        file.write_all(paste.data.as_bytes())
            .and_then(|()| Ok(paste_url))
    })
    .then(|res| match res {
        Ok(paste_url) => Ok(HttpResponse::Ok()
            .content_type("text/plain")
            .body(paste_url)),
        Err(_) => Ok(HttpResponse::InternalServerError().into()),
    })
}

fn send_paste(
    config: web::Data<Config>,
    paste_name: web::Path<String>,
) -> impl Future<Item = HttpResponse, Error = actix_web::Error> {
    web::block(move || {
        let file_path = format!("{}/{}", config.paste_dir, paste_name);
        fs::read_to_string(file_path)
    })
    .then(|res| match res {
        Ok(contents) => Ok(HttpResponse::Ok().content_type("text/plain").body(contents)),
        Err(_) => Ok(HttpResponse::NotFound().into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, http};
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;
    use std::str;

    fn make_test_config(paste_dir: &str) -> Config {
        Config {
            paste_dir: String::from(paste_dir),
            url_base: String::from("https://testurl"),
        }
    }

    #[test]
    fn get_paste_ok() {
        let test_dir = TempDir::new().unwrap();
        let config = make_test_config(test_dir.path().to_str().unwrap());

        let data = web::Data::new(config.clone());
        let mut app = test::init_service(App::new()
            .register_data(data)
            .route("/{filename}", web::get().to_async(send_paste)));

        // Write a test paste
        let paste_name = "/testpaste";
        let paste_content = b"plain text paste contents\nwith a newline";
        let mut file = File::create(config.paste_dir + paste_name).unwrap();
        file.write_all(paste_content).unwrap();

        let req = test::TestRequest::get().uri(paste_name).to_request();
        let resp = test::call_service(&mut app, req);

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");

        let resp_body = test::read_body(resp);
        assert_eq!(paste_content[..], resp_body);
    }

    #[test]
    fn get_paste_not_ok() {
        let test_dir = TempDir::new().unwrap();
        let config = make_test_config(test_dir.path().to_str().unwrap());
        let data = web::Data::new(config.clone());
        let mut app = test::init_service(App::new()
            .register_data(data)
            .route("/{filename}", web::get().to_async(send_paste)));
        let non_existent_paste = "/hebele";

        let req = test::TestRequest::get().uri(non_existent_paste).to_request();
        let resp = test::call_service(&mut app, req);

        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn post_paste_short_ascii() {
        let test_dir = TempDir::new().unwrap();
        let config = make_test_config(test_dir.path().to_str().unwrap());

        let data = web::Data::new(config.clone());
        let mut app = test::init_service(
            App::new()
                .register_data(data)
                .service(new_paste),
        );

        let paste_content = "hebele hubele\nbubele mubele\n";
        let req = test::TestRequest::post().header("content-type", "application/x-www-form-urlencoded")
            .set_payload(format!("data={}", paste_content))
            .to_request();
        let resp = test::call_service(&mut app, req);

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain");

        let resp_body = test::read_body(resp);
        let paste_url = str::from_utf8(&resp_body).unwrap();
        assert!(paste_url.starts_with(&config.url_base));

        let (_, paste_id) = paste_url.split_at(config.url_base.len());
        // Line above gets the paste id with a preceding slash, which is required for the next line to work.
        let file_content = fs::read_to_string(config.paste_dir + paste_id).unwrap();
        assert_eq!(paste_content, file_content);
    }
}
