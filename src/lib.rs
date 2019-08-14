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
    // TODO: Put more thought onto error handling and returned status codes
    web::block(move || {
        let file_path = format!("{}/{}", config.paste_dir, paste_name);
        // XXX: Is path traversal attack possible?
        fs::read_to_string(file_path)
    })
    .then(|res| match res {
        Ok(contents) => Ok(HttpResponse::Ok().content_type("text/plain").body(contents)),
        Err(_) => Ok(HttpResponse::NotFound().into()),
    })
}
