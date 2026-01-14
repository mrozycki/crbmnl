use std::{iter, str::FromStr};

use actix_web::{get, post, App, HttpRequest, HttpResponse, HttpServer, Responder};
use image::{GrayImage, Luma};
use imageproc::rect::Rect;
use itertools::Itertools;
use tracing::info;
use url::Url;

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum SpecialFunction {
    None,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DisplayResponse {
    filename: String,
    #[serde(default)]
    firmware_url: Option<Url>,
    image_url: Url,
    image_url_timeout: u16,
    refresh_rate: u16,
    special_function: SpecialFunction,
    #[serde(default)]
    reset_firmware: bool,
    #[serde(default)]
    update_firmware: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct SetupResponse {
    api_key: String,
    friendly_id: String,
    image_url: Url,
    message: String,
    status: u16,
}

#[get("/api/setup")]
async fn setup(req: HttpRequest) -> impl Responder {
    info!("setup called: {:?}", req.headers());
    HttpResponse::Ok().json(SetupResponse {
        api_key: "Zwy0Sv5zD9XnY-Ug3d7_5g".to_string(),
        friendly_id: "ABC123".to_string(),
        image_url: Url::from_str("http://192.168.0.32:8080/static/rover.bmp").unwrap(),
        message: "Dupa".to_string(),
        status: 200,
    })
}

#[get("/api/display")]
async fn display(req: HttpRequest) -> impl Responder {
    let battery_voltage = req
        .headers()
        .get("battery-voltage")
        .and_then(|v| str::from_utf8(v.as_bytes()).ok())
        .and_then(|v| v.parse::<f64>().ok())
        .map(|v| v.clamp(0.0, 4.5).div_euclid(0.45) * 10.0);

    info!(
        "display called (battery: {:?}): {:?}",
        battery_voltage,
        req.headers()
    );
    HttpResponse::Ok().json(DisplayResponse {
        filename: "rover.bmp".to_string(),
        firmware_url: Some(
            Url::from_str("https://trmnl.s3.us-east-2.amazonaws.com/FW1.5.4.bin").unwrap(),
        ),
        image_url: Url::from_str("http://192.168.0.32:8080/static/rover.bmp").unwrap(),
        image_url_timeout: 60,
        refresh_rate: 60,
        special_function: SpecialFunction::None,
        update_firmware: false,
        reset_firmware: false,
    })
}

#[get("/static/rover.bmp")]
async fn static_image() -> impl Responder {
    info!("image requested");

    let mut image = GrayImage::new(800, 480);
    imageproc::drawing::draw_filled_rect_mut(
        &mut image,
        Rect::at(0, 0).of_size(800, 480),
        Luma([255]),
    );
    imageproc::drawing::draw_filled_rect_mut(
        &mut image,
        Rect::at(350, 190).of_size(100, 100),
        Luma([0]),
    );

    let data = image
        .as_raw()
        .iter()
        .map(|p| (*p > 128) as u8)
        .tuples()
        .map(|(a, b, c, d, e, f, g, h)| {
            (a << 7) + (b << 6) + (c << 5) + (d << 4) + (e << 3) + (f << 2) + (g << 1) + h
        })
        .collect_vec();

    let image = iter::empty()
        .chain(b"BM") // BMP magic bytes
        .chain(&[0xbe, 0xbb, 0x00, 0x00]) // file size (48062 bytes)
        .chain(&[0; 4]) // reserved fields, always 0
        .chain(&[0x3e, 0x00, 0x00, 0x00]) // raw data offset (62 bytes)
        .chain(&[0x28, 0x00, 0x00, 0x00]) // DIB header size (40 bytes)
        .chain(&[0x20, 0x03, 0x00, 0x00]) // image width (800 pixels)
        .chain(&[0xe0, 0x01, 0x00, 0x00]) // image height (480 pixels)
        .chain(&[0x01, 0x00]) // number of color planes (must be 1)
        .chain(&[0x01, 0x00]) // bit depth (1 bit)
        .chain(&[0; 4]) // compression type (none)
        .chain(&[0x80, 0xbb, 0x00, 0x00]) // image size (48000 bytes)
        .chain(&[0; 4]) // horizontal resolution (ignored)
        .chain(&[0; 4]) // vertical resolution (ignored)
        .chain(&[0x02, 0x00, 0x00, 0x00]) // color palette size (2 colors)
        .chain(&[0x02, 0x00, 0x00, 0x00]) // number of important colors (2 colors)
        .chain(&[0x00, 0x00, 0x00, 0x00]) // color palette, color 0: black
        .chain(&[0xff, 0xff, 0xff, 0x00]) // color palette, color 1: white
        .chain(&data)
        .copied()
        .collect_vec();

    HttpResponse::Ok().content_type("image/bmp").body(image)
}

#[post("/api/logs")]
async fn logs(req_body: String) -> impl Responder {
    info!("logs called: {req_body}");
    HttpResponse::Ok().body(())
}

#[get("/api/{tail:.*}")]
async fn any(req: HttpRequest) -> impl Responder {
    info!("other called: {}, {:?}", req.path(), req.headers());
    HttpResponse::Ok().body(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    HttpServer::new(|| {
        App::new()
            .service(setup)
            .service(display)
            .service(logs)
            .service(any)
            .service(static_image)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
