use std::{iter, str::FromStr};

use ab_glyph::FontRef;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use chrono::{Datelike, NaiveDate, Timelike, Utc};
use crbmnl::{
    calendar::{Calendar, DateMaybeTime},
    config::Config,
    temperature::Temperature,
};
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
        filename: "render.bmp".to_string(),
        firmware_url: None,
        image_url: Url::from_str("http://192.168.0.32:8080/render.bmp").unwrap(),
        image_url_timeout: 60,
        refresh_rate: 60,
        special_function: SpecialFunction::None,
        update_firmware: false,
        reset_firmware: false,
    })
}

fn format_date(date: NaiveDate) -> String {
    let month = match date.month() {
        1 => "stycznia",
        2 => "lutego",
        3 => "marca",
        4 => "kwietnia",
        5 => "maja",
        6 => "czerwca",
        7 => "lipca",
        8 => "sierpnia",
        9 => "września",
        10 => "października",
        11 => "listopada",
        12 => "grudnia",
        _ => "???",
    };

    format!("{} {} {}", date.day(), month, date.year())
}

#[get("/render.bmp")]
async fn render(config: web::Data<Config>) -> impl Responder {
    info!("image requested");

    let mut image = GrayImage::new(800, 480);
    imageproc::drawing::draw_filled_rect_mut(
        &mut image,
        Rect::at(0, 0).of_size(800, 480),
        Luma([255]),
    );
    let now = Utc::now().with_timezone(&config.timezone);
    let today = now.date_naive();
    let font_normal = FontRef::try_from_slice(include_bytes!("../../Roboto-Light.ttf")).unwrap();
    let font_bold = FontRef::try_from_slice(include_bytes!("../../Roboto-Bold.ttf")).unwrap();
    imageproc::drawing::draw_text_mut(
        &mut image,
        Luma([0]),
        10,
        10,
        54.0,
        &font_bold,
        &format_date(today),
    );

    let events = Calendar::new(config.get_ref().clone())
        .get_next_n_days(14)
        .await
        .unwrap();

    let mut start = 91;
    let font_size = 28.0;
    let line_height = (1.5 * font_size) as i32;
    events
        .into_iter()
        .chunk_by(|e| e.start.date())
        .into_iter()
        .sorted_by(|(d1, _), (d2, _)| d1.cmp(d2))
        .for_each(|(date, events)| {
            if start > 480 - 3 * line_height {
                return;
            }
            if date != today {
                imageproc::drawing::draw_text_mut(
                    &mut image,
                    Luma([0]),
                    10,
                    start,
                    font_size,
                    &font_bold,
                    &format_date(date),
                );
                start += line_height;
            }

            events.for_each(|e| {
                if start > 480 - 2 * line_height {
                    return;
                }
                let text = if let (DateMaybeTime::DateTime(start), DateMaybeTime::DateTime(end)) =
                    (e.start, e.end)
                {
                    format!(
                        "{}:{:02}-{}:{:02} {}",
                        start.hour(),
                        start.minute(),
                        end.hour(),
                        end.minute(),
                        e.summary
                    )
                } else {
                    e.summary
                };
                imageproc::drawing::draw_text_mut(
                    &mut image,
                    Luma([0]),
                    10,
                    start,
                    font_size,
                    &font_normal,
                    &text,
                );
                start += line_height;
            });
            start += line_height;
        });

    let temperature = Temperature::new(config.get_ref().clone());
    let temperature_data = temperature.get_data().await.unwrap();
    let main_temp_text = format!("{:.1}°C", temperature_data.primary.temperature);
    let main_temp_text_font_size = 108.0;
    let (width, _) =
        imageproc::drawing::text_size(main_temp_text_font_size, &font_normal, &main_temp_text);
    imageproc::drawing::draw_text_mut(
        &mut image,
        Luma([0]),
        800 - 10 - width as i32,
        10,
        main_temp_text_font_size,
        &font_normal,
        &main_temp_text,
    );

    let mut start = (main_temp_text_font_size * 1.5) as i32 + 10;
    temperature_data
        .secondaries
        .into_iter()
        .for_each(|(name, data)| {
            let text = format!("{name}: {:.1}°C, {:.0}%", data.temperature, data.humidity);
            let (width, _) = imageproc::drawing::text_size(font_size, &font_normal, &text);
            imageproc::drawing::draw_text_mut(
                &mut image,
                Luma([0]),
                800 - 10 - width as i32,
                start,
                font_size,
                &font_normal,
                &text,
            );
            start += line_height;
        });

    let generated = format!(
        "Wygenerowano: {}",
        Utc::now().with_timezone(&config.timezone)
    );
    let (width, _) = imageproc::drawing::text_size(font_size, &font_normal, &generated);
    imageproc::drawing::draw_text_mut(
        &mut image,
        Luma([0]),
        800 - 10 - width as i32,
        480 - line_height,
        font_size,
        &font_normal,
        &generated,
    );

    imageproc::compose::flip_vertical_mut(&mut image);

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

    let settings = config::Config::builder()
        .add_source(config::File::with_name("crbmnl"))
        .build()
        .expect("failed to load config");
    let config = settings
        .try_deserialize::<crbmnl::config::Config>()
        .expect("failed to deserialize config");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .service(setup)
            .service(display)
            .service(logs)
            .service(any)
            .service(render)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
