use std::env;

use reqwest::header::{HeaderMap, HeaderValue};
use tokio::task::JoinSet;

use crate::providers::{Class, FlightRequest, FlightResponse, Provider, Stops};

#[derive(serde::Deserialize)]
struct GoogleFlightsResponse {
    best_flights: Vec<FlightGroup>,
    other_flights: Vec<FlightGroup>,
}

#[derive(serde::Deserialize)]
struct FlightGroup {
    flights: Vec<FlightInfo>,
    total_duration: u32,
    price: f32,
}

#[derive(serde::Deserialize)]
struct FlightInfo {
    departure_airport: AirportInfo,
    arrival_airport: AirportInfo,
    airline: String,
}

#[derive(serde::Deserialize)]
struct AirportInfo {
    name: String,
    id: String,
    time: String,
}

#[derive(serde::Deserialize)]
pub struct GoogleFlightsConfig {
    pub country: String,
    pub currency: String,
    pub url: String,
}

pub struct Client {
    client: reqwest::Client,
    cfg: GoogleFlightsConfig,
}

impl Client {
    pub fn new(cfg: GoogleFlightsConfig) -> Result<Client, Box<dyn std::error::Error>> {
        let api_key = env::var("GOOGLE_FLIGHTS_API_KEY")?;

        let mut headers = HeaderMap::new();
        headers.insert("api_key", HeaderValue::from_str(&api_key)?);
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Client { cfg, client })
    }

    fn request(&self) -> reqwest::RequestBuilder {
        self.client.get(&self.cfg.url).query(&[
            ("engine", "google"),
            ("hl", "en"),
            ("currency", self.cfg.currency.as_str()),
            ("gl", self.cfg.country.as_str()),
        ])
    }

    fn build_queries(req: &FlightRequest) -> Vec<Vec<(&str, String)>> {
        let mut base: Vec<(&str, String)> = vec![
            ("type", "2".to_string()),
            ("departure_id", req.origin.id.clone()),
            ("arrival_id", req.dest.id.clone()),
            ("adults", req.passengers.to_string()),
            (
                "travel_class",
                match req.class {
                    Class::Economy => "1",
                    Class::Premium => "2",
                    Class::Business => "3",
                    Class::First => "4",
                }
                .to_string(),
            ),
            (
                "stops",
                match req.stops {
                    Stops::Any => "0",
                    Stops::None => "1",
                    Stops::MaxOne => "2",
                    Stops::MaxTwo => "3",
                }
                .to_string(),
            ),
        ];

        if req.carry_on_bags > 0 {
            base.push(("bags", req.carry_on_bags.to_string()));
        }

        let mut queries = Vec::new();
        for date in &req.date {
            let mut query = base.clone();
            query.push(("outbound_date", date.date.to_string()));
            if let Some(times) = &date.time {
                let departure_time = times
                    .departure
                    .as_ref()
                    .map(|t| (t.start_hour, t.end_hour))
                    .unwrap_or((0, 23));

                let arrival_time = times
                    .arrival
                    .as_ref()
                    .map(|t| (t.start_hour, t.end_hour))
                    .unwrap_or((0, 23));

                let times_param = format!(
                    "{},{},{},{}",
                    departure_time.0, departure_time.1, arrival_time.0, arrival_time.1
                );
                query.push(("outbound_times", times_param));
            }
            queries.push(query);
        }
        queries
    }

    fn convert(resp: GoogleFlightsResponse, req: &FlightRequest) -> Vec<FlightResponse> {
        resp.best_flights
            .into_iter()
            .chain(resp.other_flights)
            .filter_map(|group| {
                let first = group.flights.first()?;
                let last = group.flights.last()?;

                let date_str = first.departure_airport.time.split(' ').next()?;
                let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;

                Some(FlightResponse {
                    flight: FlightRequest {
                        origin: super::Airport {
                            id: first.departure_airport.id.clone(),
                            name: Some(first.departure_airport.name.clone()),
                        },
                        dest: super::Airport {
                            id: last.arrival_airport.id.clone(),
                            name: Some(last.arrival_airport.name.clone()),
                        },
                        date: vec![super::Date { date, time: None }],
                        class: req.class,
                        stops: req.stops,
                        passengers: req.passengers,
                        carry_on_bags: req.carry_on_bags,
                        duration: Some(chrono::Duration::minutes(group.total_duration as i64)),
                    },
                    company: first.airline.clone(),
                    price: group.price,
                })
            })
            .collect()
    }
}

impl Provider for Client {
    async fn search(
        &self,
        req: super::FlightRequest,
    ) -> Result<Vec<FlightResponse>, Box<dyn std::error::Error>> {
        let queries = Self::build_queries(&req);
        let mut join_set = JoinSet::new();

        for query in queries {
            let req = self.request().query(&query);
            join_set.spawn(async move { req.send().await?.json::<GoogleFlightsResponse>().await });
        }

        let res = join_set
            .join_all()
            .await
            .into_iter()
            .collect::<Result<Vec<GoogleFlightsResponse>, _>>()?
            .into_iter()
            .flat_map(|r| Self::convert(r, &req))
            .collect();

        Ok(res)
    }

    async fn search_roundtrip(
        &self,
        req: super::RoundTripRequest,
    ) -> Result<super::RoundTripResponse, Box<dyn std::error::Error>> {
        let (inbound, outbound) = tokio::join!(self.search(req.inbound), self.search(req.outbound));
        Ok(super::RoundTripResponse {
            inbound: inbound?,
            outbound: outbound?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{Airport, Date, TimeRange, Times};
    use chrono::NaiveDate;
    use std::collections::HashMap;

    #[test]
    fn test_build_query() {
        let req = crate::providers::FlightRequest {
            origin: Airport {
                id: "LIS".to_string(),
                name: None,
            },
            dest: Airport {
                id: "MAD".to_string(),
                name: None,
            },
            date: vec![
                Date {
                    date: NaiveDate::from_ymd_opt(2026, 9, 26).unwrap(),
                    time: Some(Times {
                        departure: Some(TimeRange {
                            start_hour: 8,
                            end_hour: 14,
                        }),
                        arrival: None,
                    }),
                },
                Date {
                    date: NaiveDate::from_ymd_opt(2026, 9, 26).unwrap(),
                    time: Some(Times {
                        departure: Some(TimeRange {
                            start_hour: 19,
                            end_hour: 23,
                        }),
                        arrival: None,
                    }),
                },
            ],
            class: Class::Economy,
            stops: Stops::None,
            passengers: 2,
            carry_on_bags: 1,
            duration: None,
        };

        let queries = Client::build_queries(&req);
        assert_eq!(queries.len(), 2);

        // Shared params both queries must have
        let expected_base: Vec<(&str, &str)> = vec![
            ("type", "2"),
            ("departure_id", "LIS"),
            ("arrival_id", "MAD"),
            ("adults", "2"),
            ("travel_class", "1"),
            ("stops", "1"),
            ("bags", "1"),
        ];

        for query in &queries {
            let map: HashMap<&str, &str> = query.iter().map(|(k, v)| (*k, v.as_str())).collect();

            for (key, val) in &expected_base {
                assert_eq!(map.get(key), Some(val));
            }
        }

        // First query: departure 8-14, arrival 0-23
        let query: HashMap<&str, &str> = queries[0].iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(query.get("outbound_date"), Some(&"2026-09-26"));
        assert_eq!(query.get("outbound_times"), Some(&"8,14,0,23"));

        // Second query: departure 19-23, arrival 0-23
        let query: HashMap<&str, &str> = queries[1].iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(query.get("outbound_date"), Some(&"2026-09-26"));
        assert_eq!(query.get("outbound_times"), Some(&"19,23,0,23"));
    }
}
