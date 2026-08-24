pub mod google_flights;

#[derive(Clone, Copy)]
pub enum Class {
    Economy,
    Premium,
    Business,
    First,
}

#[derive(Clone, Copy)]
pub enum Stops {
    None,
    Any,
    MaxOne,
    MaxTwo,
}

pub struct RoundTripRequest {
    pub inbound: FlightRequest,
    pub outbound: FlightRequest,
    pub minimum_days: usize,
}

pub struct RoundTripResponse {
    pub inbound: Vec<FlightResponse>,
    pub outbound: Vec<FlightResponse>,
}

#[derive(Clone)]
pub struct FlightRequest {
    pub origin: Airport,
    pub dest: Airport,
    pub date: Vec<Date>, // Allow for multiple possible dates

    pub class: Class,
    pub stops: Stops,
    pub passengers: usize,
    pub carry_on_bags: usize,

    pub duration: Option<chrono::Duration>,
}

pub struct FlightResponse {
    pub flight: FlightRequest,
    pub company: String,
    pub price: f32,
}

#[derive(Clone)]
pub struct Airport {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Clone)]
pub struct Date {
    pub date: chrono::NaiveDate,
    pub time: Option<Times>,
}

#[derive(Clone)]
pub struct Times {
    pub departure: Option<TimeRange>,
    pub arrival: Option<TimeRange>,
}

#[derive(Clone)]
pub struct TimeRange {
    pub start_hour: usize,
    pub end_hour: usize,
}

pub trait Provider {
    fn search(
        &self,
        req: FlightRequest,
    ) -> impl std::future::Future<Output = Result<Vec<FlightResponse>, Box<dyn std::error::Error>>>;
    fn search_roundtrip(
        &self,
        req: RoundTripRequest,
    ) -> impl Future<Output = Result<RoundTripResponse, Box<dyn std::error::Error>>>;
}
