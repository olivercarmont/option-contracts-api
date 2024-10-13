# Option Contracts Lambda Function

This Lambda function retrieves relevant option contracts for a given stock ticker using the Polygon.io API. It's written in Rust and can be deployed as an AWS Lambda function.

### Input

- `ticker_symbol`: The stock ticker symbol (e.g., "AAPL" for Apple Inc.)
- `api_key`: Your Polygon.io API key
- `limit`: The maximum number of contracts to retrieve (default: 10)
- `days_forward`: The number of days in the future to look for contracts (default: 30)
- `contract_type`: The type of option contract to retrieve ("call" or "put")

### Example Lambda Function Call

You can invoke the Lambda function with the following JSON input:

```json
{
    "ticker_symbol": "AAPL",
    "api_key": "YOUR_POLYGON_API_KEY",
    "limit": "20",
    "days_forward": "60",
    "contract_type": "put"
}
```

### Example Output

The function returns a JSON response with an array of option contracts. Each contract is represented as a JSON object with the following structure:

```json
{
    "contract_type": "put",
    "expiration_date": "2024-10-18",
    "implied_volatility": "239.97%",
    "open_interest": "1447",
    "premium": "N/A",
    "strike_price": "100",
    "ticker": "O:AAPL241018P00100000"
}
```

## Set Up and Deploying

To set up and deploy this Lambda function, follow these steps based on the [AWS Lambda Rust deployment guide](https://docs.aws.amazon.com/lambda/latest/dg/rust-package.html):

1. Ensure you have Rust installed on your system.
2. Install the AWS CLI version 2.
3. Install Cargo Lambda:
   ```
   pip3 install cargo-lambda
   ```
4. Create the package structure:
   ```
   cargo lambda new option-contracts
   ```
5. Replace the generated code in `src/main.rs` with the code for this function.
6. Build the Lambda function:
   ```
   cargo lambda build --release
   ```
7. Deploy the function using Cargo Lambda:
   ```
   cargo lambda deploy option-contracts
   ```

Alternatively, you can deploy using the AWS CLI or AWS SAM CLI as described in the AWS documentation.

### Enabling Function URL

To make your Lambda function accessible via an HTTP endpoint, you can enable Function URL in the AWS Lambda console:

1. Navigate to your Lambda function in the AWS Console.
2. Go to the "Configuration" tab.
3. Click on "Function URL" in the left sidebar.
4. Click "Create function URL" and configure the settings as follows:
   - Auth type: NONE
   - Configure cross-origin resource sharing (CORS): Enabled
   - Allow origin: * (or specify your allowed origins)
   - Allow headers: Content-Type, ticker_symbol, api_key, limit, days_forward, contract_type
   - Allow methods: POST
   - Allow credentials: Yes
5. Save the changes to get a unique URL for your function.

These settings will make your Function URL publicly accessible, enable CORS, and allow the necessary headers for the function to work properly.

## Invoking the Function

When invoking the function through the Function URL, you need to provide the input parameters as headers in your HTTP POST request. Here's an example using curl:

```bash
curl -X POST 'https://your-function-url.lambda-url.region.on.aws/' \
  -H 'Content-Type: application/json' \
  -H 'ticker_symbol: AAPL' \
  -H 'api_key: YOUR_POLYGON_API_KEY' \
  -H 'limit: 20' \
  -H 'days_forward: 60' \
  -H 'contract_type: put'
```

Make sure to replace `https://your-function-url.lambda-url.region.on.aws/` with your actual Function URL, and `YOUR_POLYGON_API_KEY` with your actual Polygon.io API key.

## Important Note

This function requires a Polygon.io API key with access to options data. Make sure you have the Pro plan or higher on Polygon.io to access the necessary endpoints.