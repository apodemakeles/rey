use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context};
use clap::Parser;
use flexi_logger::{FlexiLoggerError, Logger};
use reqwest::Url;
use tokio::signal::ctrl_c;
use tokio::sync::Notify;

use rey::arg::Args;
use rey::client::ClientBuilder;
use rey::work::Work;

macro_rules! unwrap_or_exit {
	($expr:expr) => {
		match $expr {
			Ok(val) => val,
			Err(err) => {
				eprintln!("{:?}", err);
				std::process::exit(1);
			}
		}
	};
}

#[tokio::main]
async fn main() {
	unwrap_or_exit!(init_logger().context("fail to statup logger"));
	let args = Args::parse();
	let body: Vec<u8>;
	if let Some(body_str) = args.body {
		body = body_str.into_bytes();
	} else if let Some(file) = args.body_file {
		body = unwrap_or_exit!(tokio::fs::read(file)
			.await
			.map_err(|err| { anyhow!("invalid BODY FILE: {}", err) }));
	} else {
		body = vec![];
	}
	let body: &'static [u8] = Box::leak(body.into_boxed_slice());
	let client_builder = ClientBuilder {
		headers: args.headers,
		timeout: if args.timeout > 0 {
			Some(Duration::from_secs(args.timeout))
		} else {
			None
		},
		content_type: args.content_type_header,
		accept: args.accept_header,
		user_agent: args.user_agent_header,
		proxy: args.proxy_address,
		host: args.host,
		disable_redirect: args.disable_redirect,
	};
	let work = Work {
		client_builder,
		url: unwrap_or_exit!(args.url.parse::<Url>().context("invalid url")),
		method: args.method,
		workers: args.workers,
		auth: args.basic_auth,
		total_requests: args.requests,
		rate_limit: args.rate_limit,
		body,
	};
	let notify = Arc::new(Notify::new());
	let cancel = notify.clone();
	// todo: instead by pending()
	let max_duration = args
		.max_duration
		.unwrap_or(Duration::from_secs(60 * 60 * 24));
	tokio::spawn(async move {
		tokio::select! {
			_ = tokio::time::sleep(max_duration)=>{}
			_ = ctrl_c()=>{}
		}
		notify.notify_one();
	});

	// execute
	let start = Instant::now();
	let report = unwrap_or_exit!(work.execute(cancel).await);
	let reporter = report.into_report(start.elapsed());
	reporter.print();
}

fn init_logger() -> Result<(), FlexiLoggerError> {
	Logger::try_with_env_or_str("warn")?
		.format(flexi_logger::colored_detailed_format)
		.start()?;
	Ok(())
}
