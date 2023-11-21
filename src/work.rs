use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use http::Method;
use log::info;
use reqwest::{Body, Client, Url};
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Notify;
use tokio::time::Instant;

use crate::report::Reporter;

#[derive(Debug)]
struct SourceStat {
	pub duration: Duration,
	pub status_code: u16,
	pub content_length: u64,
}

type RequestResult = Result<SourceStat, reqwest::Error>;

#[derive(Debug, Clone, PartialEq)]
pub struct BasicAuth {
	pub username: String,
	pub password: Option<String>,
}

struct Worker<B>
where
	B: Into<Body> + Copy,
{
	url: Url,
	method: Method,
	basic_auth: Option<BasicAuth>,
	rate_limit: Option<f64>,
	body: B,
	requests: u64,
	client: Arc<Client>,
	sender: Sender<RequestResult>,
}

impl<B> Worker<B>
where
	B: Into<Body> + Copy,
{
	async fn make_request(&self) -> RequestResult {
		let start = Instant::now();

		// build
		let client = self.client.clone();
		let method = self.method.clone();
		let url = self.url.clone();
		let mut builder = client.request(method, url);
		if let Some(auth) = self.basic_auth.clone() {
			builder = builder.basic_auth(auth.username, auth.password);
		}
		// request
		let request = builder.body(self.body).build()?;
		let response = client.execute(request).await?;
		let status_code = response.status().as_u16();
		let content_length = response.content_length().unwrap_or(0);
		let _res = response.bytes().await?;
		Ok(SourceStat {
			duration: start.elapsed(),
			status_code,
			content_length,
		})
	}

	async fn execute(&self) {
		let interval = self
			.rate_limit
			.map(|qps| (1000000_f64 / qps).floor() as u64);
		for _ in 0..self.requests {
			if let Some(interval) = interval {
				tokio::time::sleep(Duration::from_micros(interval)).await;
			}
			let result = self.make_request().await;
			let sender = self.sender.clone();
			if let Err(error) = sender.send(result).await {
				info!("worker interrupt due to error:{}", error);
				return;
			}
		}
	}
}

pub struct Work<C, B>
where
	C: TryInto<Client, Error = anyhow::Error>,
	B: Into<Body> + Copy + Send + Sync + 'static,
{
	pub client_builder: C,
	pub url: Url,
	pub method: Method,
	pub auth: Option<BasicAuth>,
	pub workers: u16,
	pub total_requests: u64,
	pub rate_limit: Option<f64>,
	pub body: B,
}

impl<C, B> Work<C, B>
where
	C: TryInto<Client, Error = anyhow::Error>,
	B: Into<Body> + Copy + Send + Sync + 'static,
{
	pub async fn execute(self, cancel: Arc<Notify>) -> anyhow::Result<Reporter> {
		let client = Arc::new(self.client_builder.try_into()?);
		let requests = self.total_requests / (self.workers as u64);
		let (sender, mut receiver) = channel(self.workers as usize);
		for _ in 0..self.workers {
			let worker = Worker {
				url: self.url.clone(),
				method: self.method.clone(),
				basic_auth: self.auth.clone(),
				rate_limit: self.rate_limit,
				requests,
				client: client.clone(),
				sender: sender.clone(),
				body: self.body,
			};
			tokio::spawn(async move {
				worker.execute().await;
			});
		}
		drop(sender);

		let mut total_requests = 0_u64;
		let mut success_requests = 0_u64;
		let mut durations = vec![];
		let mut status_codes = vec![];
		let mut size_total = 0_u64;
		let mut error_dist = HashMap::new();

		loop {
			tokio::select! {
				_ = cancel.notified()=>{
					info!("receive cancel signal");
					receiver.close();
					break;
				}
				msg = receiver.recv() =>{
					match msg{
						None => {
							info!("all sender of worker been closed, finish receiving source stats");
							break;
						},
						Some(result)=>{
							total_requests += 1;
							match result{
								Err(err)=>*error_dist.entry(err.to_string()).or_insert(0) += 1,
								Ok(stat)=>{
									success_requests += 1;
									durations.push(stat.duration.as_secs_f64());
									status_codes.push(stat.status_code);
									size_total += stat.content_length;
								}
							}
						}
					}
				}
			}
		}

		Ok(Reporter {
			total_requests,
			success_requests,
			durations,
			status_codes,
			size_total,
			error_dist,
		})
	}
}
