use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use reqwest::Client;
use chrono::{Local, Duration};
use futures::future::join_all;
use urlencoding::encode;

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
struct Payload {
    ticker_symbol: Option<String>,
    api_key: Option<String>,
    limit: Option<String>,
    days_forward: Option<String>,
    contract_type: Option<String>,
}

#[derive(Serialize)]
struct Response {
    req_id: String,
    response: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
enum RequestData {
    Payload(Payload),
    Headers(Headers),
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct Headers {
    headers: HeaderValues,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct HeaderValues {
    ticker_symbol: Option<String>,
    api_key: Option<String>,
    limit: Option<String>,
    days_forward: Option<String>,
    contract_type: Option<String>,
}

async fn get_relevant_option_contracts(
    client: &Client,
    api_key: &str,
    ticker_symbol: &str,
    limit: &str,
    days_forward: &str,
    contract_type: &str,
) -> Result<Vec<String>, Error> {
    let base_url = "https://api.polygon.io/v3/reference/options/contracts";
    let today = Local::now().date_naive();
    let days_forward_int: i64 = days_forward.parse().unwrap_or(30);
    let future_date = today + Duration::days(days_forward_int);

    let response = client
        .get(base_url)
        .query(&[
            ("apiKey", api_key),
            ("underlying_ticker", ticker_symbol),
            ("limit", limit),
            ("order", "asc"),
            ("sort", "expiration_date"),
            ("expiration_date.gte", &today.format("%Y-%m-%d").to_string()),
            ("expiration_date.lte", &future_date.format("%Y-%m-%d").to_string()),
            ("contract_type", contract_type),
        ])
        .send()
        .await?;

    let status = response.status(); // Capture the status code before consuming the response

    if status.is_success() {
        let data: Value = response.json().await?;
        let tickers: Vec<String> = data["results"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|contract| contract["ticker"].as_str().map(|s| s.to_string()))
            .collect();
        Ok(tickers)
    } else {
        let error_text = response.text().await?;
        println!("Error fetching contracts: Status code {}, Response: {}", status, error_text);
        Ok(Vec::new())
    }
}


async fn get_contract_details(
    client: &Client,
    api_key: &str,
    underlying_asset: &str,
    option_ticker: &str,
) -> Result<Value, Error> {
    let encoded_option_ticker = encode(option_ticker);
    let base_url = format!(
        "https://api.polygon.io/v3/snapshot/options/{}/{}",
        underlying_asset, encoded_option_ticker
    );

    let response = client
        .get(&base_url)
        .query(&[("apiKey", api_key)])
        .send()
        .await?;

    let status = response.status(); // Capture the status code before consuming the response

    if status.is_success() {
        let data: Value = response.json().await?;
        Ok(data["results"].clone())
    } else {
        let error_text = response.text().await?;
        println!(
            "Error fetching details for {}: Status code {}, Response: {}",
            option_ticker, status, error_text
        );
        Ok(Value::Null)
    }
}

async fn function_handler(event: LambdaEvent<Value>) -> Result<Response, Error> {
    println!("Received event: {:?}", event);

    let (payload, request_id) = if let Some(query_params) = event.payload.get("queryStringParameters") {
        // Parameters are in query string
        let payload = extract_parameters_from_value(query_params);
        let request_id = event
            .payload
            .get("requestContext")
            .and_then(|rc| rc.get("requestId"))
            .and_then(|id| id.as_str())
            .unwrap_or(&event.context.request_id)
            .to_string();
        (payload, request_id)
    } else if let Some(headers) = event.payload.get("headers") {
        // Parameters are in headers
        let payload = extract_parameters_from_value(headers);
        let request_id = event
            .payload
            .get("requestContext")
            .and_then(|rc| rc.get("requestId"))
            .and_then(|id| id.as_str())
            .unwrap_or(&event.context.request_id)
            .to_string();
        (payload, request_id)
    } else if let Some(body) = event.payload.get("body") {
        // Parameters are in body
        let body_str = body.as_str().unwrap_or("");
        let payload: Payload = serde_json::from_str(body_str).unwrap_or_default();
        let request_id = event
            .payload
            .get("requestContext")
            .and_then(|rc| rc.get("requestId"))
            .and_then(|id| id.as_str())
            .unwrap_or(&event.context.request_id)
            .to_string();
        (payload, request_id)
    } else {
        // Direct invocation or test event
        let payload: Payload = serde_json::from_value(event.payload.clone()).unwrap_or_default();
        (payload, event.context.request_id.clone())
    };

    // Extract parameters
    let ticker_symbol = payload.ticker_symbol.unwrap_or_else(|| "AAPL".to_string());
    let api_key = payload.api_key.unwrap_or_else(|| "YOUR_API_KEY".to_string());
    let limit = payload.limit.unwrap_or("10".to_string());
    let days_forward = payload.days_forward.unwrap_or("30".to_string());
    let contract_type = payload.contract_type.unwrap_or("call".to_string());

    println!("Using parameters:");
    println!("Ticker Symbol: {}", ticker_symbol);
    println!("API Key: {}", api_key);
    println!("Limit: {}", limit);
    println!("Days Forward: {}", days_forward);
    println!("Contract Type: {}", contract_type);

    let client = Client::new();
    let contract_tickers = get_relevant_option_contracts(
        &client,
        &api_key,
        &ticker_symbol,
        &limit,
        &days_forward,
        &contract_type,
    )
    .await?;

    println!("Retrieved contract tickers: {:?}", contract_tickers);

    // Fetch details concurrently for better performance
    let fetches = contract_tickers.iter().map(|ticker| {
        get_contract_details(&client, &api_key, &ticker_symbol, ticker)
    });

    let contracts_data = join_all(fetches).await;

    // Process and format the data
    let formatted_contracts: Vec<Value> = contracts_data
        .into_iter()
        .filter_map(|result| match result {
            Ok(contract) => {
                if contract.is_null() {
                    println!("Contract data is null.");
                    None
                } else {
                    let contract_type = contract["details"]["contract_type"]
                        .as_str()
                        .unwrap_or("N/A");
                    let expiration_date = contract["details"]["expiration_date"]
                        .as_str()
                        .unwrap_or("N/A");
                    let strike_price = contract["details"]["strike_price"]
                        .as_f64()
                        .map(|p| p.to_string())
                        .unwrap_or("N/A".to_string());
                    let implied_volatility = contract["implied_volatility"]
                        .as_f64()
                        .map(|v| format!("{:.2}%", v * 100.0))
                        .unwrap_or("N/A".to_string());
                    let open_interest = contract["open_interest"]
                        .as_u64()
                        .map(|v| v.to_string())
                        .unwrap_or("N/A".to_string());
                    let premium = contract["last_quote"]["midpoint"]
                        .as_f64()
                        .map(|p| format!("{:.2}", p))
                        .unwrap_or("N/A".to_string());
                    let ticker = contract["details"]["ticker"]
                        .as_str()
                        .unwrap_or("N/A");

                    Some(json!({
                        "contract_type": contract_type,
                        "expiration_date": expiration_date,
                        "implied_volatility": implied_volatility,
                        "open_interest": open_interest,
                        "premium": premium,
                        "strike_price": strike_price,
                        "ticker": ticker
                    }))
                }
            }
            Err(e) => {
                println!("Error fetching contract details: {}", e);
                None
            }
        })
        .collect();

    println!("Formatted contracts: {:?}", formatted_contracts);

    let resp = Response {
        req_id: request_id,
        response: serde_json::to_string(&json!({ "option_contracts": formatted_contracts }))?,
    };

    Ok(resp)
}

fn extract_parameters_from_value(value: &Value) -> Payload {
    Payload {
        ticker_symbol: value.get("ticker_symbol").and_then(|v| v.as_str()).map(|s| s.to_string()),
        api_key: value.get("api_key").and_then(|v| v.as_str()).map(|s| s.to_string()),
        limit: value.get("limit").and_then(|v| v.as_str()).map(|s| s.to_string()),
        days_forward: value.get("days_forward").and_then(|v| v.as_str()).map(|s| s.to_string()),
        contract_type: value.get("contract_type").and_then(|v| v.as_str()).map(|s| s.to_string()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(function_handler)).await
}