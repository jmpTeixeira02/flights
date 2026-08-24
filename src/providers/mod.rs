use core::time;

pub enum Class {
    Economy,
    Premium,
    Business,
    First,
}

pub enum Stops {
    None,
    Any,
    MaxOne,
    MaxTwo,
}

pub struct RoundTripRequest {
    inbound: Flight,
    outbound: Flight,
    minimum_days: usize,
}

pub struct RoundTripResponse {
    inbound: Vec<Response>,
    outbound: Vec<Response>,
}

pub struct Flight {
    origin: Airport,
    dest: Airport,
    date: Vec<Date>, // Allow to select multiple possible dates

    class: Class,
    passengers: usize,
    carry_on: bool,

    duration: Option<chrono::Duration>,
}

pub struct Response {
    flight: Flight,
    company: String,
    price: f32,
}

pub struct Airport {
    id: String,
    name: Option<String>,
}

pub struct Date {
    day: chrono::NaiveDate,
    time: Option<TimeRange>,
}

pub struct TimeRange {
    start_hour: usize,
    end_hour: usize,
}

pub trait Provider {
    fn search(req: Flight) -> Vec<Response>;
    fn search_roundtrip(req: RoundTripRequest) -> RoundTripResponse;
}
