use flights::providers::google_flights::{Client, GoogleFlightsConfig};
use flights::providers::{Airport, Class, Date, FlightRequest, Provider, Stops, TimeRange, Times};
use wiremock::matchers::{method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup() -> (MockServer, Client) {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(query_param("outbound_date", "2026-09-26"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/google_flights/one-way-morning-day.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(query_param("outbound_date", "2026-09-25"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/google_flights/one-way-noon-day-before.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    unsafe {
        std::env::set_var("GOOGLE_FLIGHTS_API_KEY", "test");
    }

    let client = Client::new(GoogleFlightsConfig {
        country: "PT".to_string(),
        currency: "EUR".to_string(),
        url: server.uri(),
    })
    .expect("error creating client");

    (server, client)
}

fn make_request(dates: Vec<Date>) -> FlightRequest {
    FlightRequest {
        origin: Airport {
            id: "LIS".into(),
            name: None,
        },
        dest: Airport {
            id: "MAD".into(),
            name: None,
        },
        date: dates,
        class: Class::Economy,
        stops: Stops::Any,
        passengers: 2,
        carry_on_bags: 2,
        duration: None,
    }
}

#[tokio::test]
async fn test_search_morning_flights() {
    let (_server, client) = setup().await;

    let results = client
        .search(make_request(vec![Date {
            date: chrono::NaiveDate::from_ymd_opt(2026, 9, 26).unwrap(),
            time: Some(Times {
                departure: Some(TimeRange {
                    start_hour: 8,
                    end_hour: 14,
                }),
                arrival: None,
            }),
        }]))
        .await
        .expect("error making requests");

    assert_eq!(results.len(), 6);
}

#[tokio::test]
async fn test_search_both_dates() {
    let (_server, client) = setup().await;

    let results = client
        .search(make_request(vec![
            Date {
                date: chrono::NaiveDate::from_ymd_opt(2026, 9, 26).unwrap(),
                time: Some(Times {
                    departure: Some(TimeRange {
                        start_hour: 8,
                        end_hour: 14,
                    }),
                    arrival: None,
                }),
            },
            Date {
                date: chrono::NaiveDate::from_ymd_opt(2026, 9, 25).unwrap(),
                time: Some(Times {
                    departure: Some(TimeRange {
                        start_hour: 12,
                        end_hour: 18,
                    }),
                    arrival: None,
                }),
            },
        ]))
        .await
        .expect("error making requests");

    assert_eq!(results.len(), 12);
}
