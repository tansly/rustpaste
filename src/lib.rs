use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use futures::Future;
use serde::Deserialize;
use std::error::Error;
use std::fs;

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

#[post("/")]
fn new_paste(config: web::Data<Config>, paste: web::Form<Paste>) -> impl Responder {
    // TODO: Generate the paste name randomly
    // TODO: Consider using multipart formdata instead of urlencoded.
    let paste_name = "pastename";
    let file_path = format!("{}/{}", config.paste_dir, paste_name);

    // TODO: Write to actual file
    println!("{}", paste.data);

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("{}/{}", config.url_base, paste_name))
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
}
