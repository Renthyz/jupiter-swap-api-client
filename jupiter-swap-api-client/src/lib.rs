use anyhow::{anyhow, Result};
use quote::{QuoteRequest, QuoteResponse};
use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use swap::{SwapInstructionsResponse, SwapInstructionsResponseInternal, SwapRequest, SwapResponse};

pub mod quote;
mod route_plan_with_metadata;
mod serde_helpers;
pub mod swap;
pub mod transaction_config;

pub const BASE_PATH: &str = "https://quote-api.jup.ag/v6";

#[derive(Clone)]
pub struct JupiterSwapApiClient {
    pub client: Client,
    pub base_path: String,
}

impl Default for JupiterSwapApiClient {
    fn default() -> Self {
        Self {
            client: Client::new(),
            base_path: BASE_PATH.to_string(),
        }
    }
}

async fn check_is_success(response: Response) -> Result<Response> {
    if !response.status().is_success() {
        return Err(anyhow!(
            "request status not ok: {}, body: {:?}",
            response.status(),
            response.text().await
        ));
    }
    Ok(response)
}

async fn check_status_code_and_deserialize<T: DeserializeOwned>(response: Response) -> Result<T> {
    check_is_success(response)
        .await?
        .json::<T>()
        .await
        .map_err(Into::into)
}

impl JupiterSwapApiClient {
    pub fn new(base_path: String, client: Client) -> Self {
        Self { base_path, client }
    }

    pub async fn quote(&self, quote_request: &QuoteRequest) -> Result<QuoteResponse> {
        let url = format!("{}/quote", self.base_path);
        let response = self.client.get(url).query(&quote_request).send().await?;
        check_status_code_and_deserialize(response).await
    }

    pub async fn swap(&self, swap_request: &SwapRequest) -> Result<SwapResponse> {
        let response = self
            .client
            .post(format!("{}/swap", self.base_path))
            .json(swap_request)
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }

    pub async fn swap_instructions(
        &self,
        swap_request: &SwapRequest,
    ) -> Result<SwapInstructionsResponse> {
        let response = self
            .client
            .post(format!("{}/swap-instructions", self.base_path))
            .json(swap_request)
            .send()
            .await?;
        check_status_code_and_deserialize::<SwapInstructionsResponseInternal>(response)
            .await
            .map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::{pubkey, pubkey::Pubkey};
    use transaction_config::TransactionConfig;

    use super::*;

    const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    const NATIVE_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

    async fn get_quote_response(client: &JupiterSwapApiClient) -> Result<QuoteResponse> {
        let quote_request = QuoteRequest {
            input_mint: NATIVE_MINT,
            output_mint: USDC_MINT,
            amount: 10000000,
            ..Default::default()
        };

        client.quote(&quote_request).await
    }

    #[tokio::test]
    async fn test_quote() {
        let client = JupiterSwapApiClient::default();

        assert!(get_quote_response(&client).await.is_ok());
    }

    #[tokio::test]
    async fn test_swap() {
        let client = JupiterSwapApiClient::default();
        let quote_response = get_quote_response(&client).await.unwrap();
        let swap_request = SwapRequest {
            user_public_key: Pubkey::default(),
            quote_response,
            config: TransactionConfig::default(),
        };

        assert!(client.swap(&swap_request).await.is_ok());
    }

    #[tokio::test]
    async fn test_swap_instructions() {
        let client = JupiterSwapApiClient::default();
        let quote_response = get_quote_response(&client).await.unwrap();
        let swap_request = SwapRequest {
            user_public_key: Pubkey::default(),
            quote_response,
            config: TransactionConfig::default(),
        };

        assert!(client.swap_instructions(&swap_request).await.is_ok());
    }
}
