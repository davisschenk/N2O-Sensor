use chrono::prelude::*;
use glob::glob;
use nmea::Nmea;
use std::env;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

type SharedNmea = Arc<Mutex<Nmea>>;

const LOG_DATA_FMT: &str = "%F_%H%M%S";

const GPS_ENV: &str = "GPS_RECEIVER";
const DIR_ENV: &str = "DATA_DIRECTORY";
const SENSOR_PORT: &str = "SENSOR_PORT";
const SENSOR_BAUD: &str = "SENSOR_BAUD";
const SENSOR_HEADER: &str = "SENSOR_HEADER";

fn get_env(name: &str) -> String {
    match env::var(name) {
        Ok(var) => var,
        Err(e) => panic!("Failed to find environment variable {name}: {e:?}"),
    }
}

fn clean_stream<I, E>(iterator: I) -> impl Iterator<Item = String>
where
    I: Iterator<Item = Result<String, E>>,
{
    iterator
        .filter_map(std::result::Result::ok)
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn refresh_gps(nmea: SharedNmea) {
    let gps_receiver = get_env(GPS_ENV);

    let input = BufReader::new(File::open(gps_receiver).expect("Failed to open gps receiver"));

    log::info!("Reading GPS Data");

    for line in clean_stream(input.lines()) {
        if let Ok(mut gps) = nmea.lock() {
            if (*gps).parse(&line).is_err() {
                log::trace!("Error parsing gps {:?}", line);
            }
        }

        thread::sleep(Duration::from_millis(1));
    }
}

fn spawn_gps(nmea: &SharedNmea) {
    let thread_nmea = nmea.clone();
    thread::spawn(move || refresh_gps(thread_nmea));
}

fn get_filename() -> PathBuf {
    let directory = PathBuf::from(get_env(DIR_ENV));

    let now: DateTime<Local> = Local::now();
    let time = now.format(LOG_DATA_FMT);

    let filename = format!("log_{time}.csv");

    directory.join(filename)
}

fn get_sensor_path() -> PathBuf {
    let pattern = get_env(SENSOR_PORT);

    let path = glob(&pattern)
        .expect("Invalid sensor port")
        .next()
        .expect("Failed to find sensor port")
        .expect("Bad");

    path
}

fn read_data<T: Write>(gps: &SharedNmea, file: &mut T) {
    let sensor_port: String = get_sensor_path().to_str().unwrap().to_owned();

    let sensor_baud: u32 = get_env(SENSOR_BAUD)
        .parse()
        .expect("Failed to parse SENSOR_BAUD");

    log::info!(
        "Opening Sensor Serial Port: {} ({})",
        sensor_port,
        sensor_baud
    );

    let port = match serialport::new(sensor_port, sensor_baud)
        .timeout(Duration::from_secs(60))
        .open()
    {
        Ok(port) => port,
        _ => panic!("Failed to open serial port for N2O Sensor"),
    };

    let buffer = BufReader::new(port);

    for mut line in clean_stream(buffer.lines()) {
        line.retain(|c| !c.is_whitespace());
        output_data(gps, file, line);
    }
}

fn output_data<T: Write>(gps: &SharedNmea, file: &mut T, line: String) {
    let mut latitude: String = String::default();
    let mut longitude: String = String::default();
    let mut altitude: String = String::default();

    if let Ok(gps_data) = gps.lock() {
        if let (Some(lat), Some(lon), Some(alt)) = (
            gps_data.latitude(),
            gps_data.longitude(),
            gps_data.altitude(),
        ) {
            latitude = format!("{lat}");
            longitude = format!("{lon}");
            altitude = format!("{alt}");
        }
    } else {
        log::error!("Failed to get GPS Mutex Lock");
        return;
    }

    log::trace!("Got Data: {}", line);
    log::trace!("Location: {}, {} ({})", latitude, longitude, altitude);

    if let Err(e) = writeln!(file, "{line},{latitude},{longitude},{altitude}") {
        log::error!("Failed to write to output file: {:?}", e);
    }
}

fn write_header<T: Write>(file: &mut T) {
    let sensor_header = get_env(SENSOR_HEADER);
    writeln!(file, "{sensor_header},latitude,longitude,altitude").unwrap();
}

fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    thread::sleep(Duration::from_secs(5));

    log::info!("Started N2O Sensor Script");

    let nmea = SharedNmea::default();

    let output_filename = get_filename();
    let mut output = File::create(&output_filename).unwrap_or_else(|_| {
        log::error!("Failed to create log: {:?}", output_filename);
        exit(1);
    });

    log::info!("Logging to {:?}", output_filename);

    spawn_gps(&nmea);
    write_header(&mut output);
    read_data(&nmea, &mut output);
}
