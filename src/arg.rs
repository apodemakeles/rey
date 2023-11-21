use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use http::{HeaderValue, Method};
use lazy_static::lazy_static;

use crate::work::BasicAuth;

lazy_static! {
	static ref VALID_METHODS: HashSet<Method> = {
		let mut set = HashSet::new();
		set.insert(Method::GET);
		set.insert(Method::POST);
		set.insert(Method::PUT);
		set.insert(Method::DELETE);
		set.insert(Method::HEAD);
		set.insert(Method::PATCH);
		set.insert(Method::OPTIONS);
		set.insert(Method::TRACE);
		set.insert(Method::CONNECT);
		set
	};
}

fn parse_duration(s: &str) -> Result<Duration, &'static str> {
	duration_str::parse(s).map_err(|_err| "not a standard duration string")
}

fn parse_method(s: &str) -> Result<Method, &'static str> {
	match s.parse::<Method>() {
		Err(_) => Err("invalid method"),
		Ok(ref method) if !VALID_METHODS.contains(method) => Err("invalid method"),
		Ok(method) => Ok(method),
	}
}

fn parse_basic_auth(s: &str) -> Result<BasicAuth, &'static str> {
	let splits: Vec<&str> = s.splitn(2, ':').collect();
	if splits.is_empty() {
		return Err("invalid username and password");
	}
	Ok(BasicAuth {
		username: splits[0].to_string(),
		password: splits.get(1).map(|password| password.to_string()),
	})
}

macro_rules! define_parse_header_fn {
	($fn_name:ident, $static_str: expr) => {
		fn $fn_name(s: &str) -> Result<HeaderValue, &'static str> {
			HeaderValue::try_from(s).map_err(|_| $static_str)
		}
	};
}

define_parse_header_fn!(parse_content_type, "invalid content-type");
define_parse_header_fn!(parse_accept, "invalid accept");
define_parse_header_fn!(parse_user_agent, "invalid user agent");
// define_parse_header_fn!(parse_host, "invalid host");

#[derive(Parser, Debug)]
#[command(version)]
pub struct Args {
	pub url: String,

	/// Name of the person to greet
	#[arg(short = 'n', default_value = "200")]
	pub requests: u64,

	/// Number of workers to run concurrently. Total number of requests cannot be smaller than the concurrency level
	#[arg(short = 'c', default_value = "50")]
	pub workers: u16,

	/// Rate limit, in queries per second (QPS) per worker
	#[arg(short = 'q', value_name = "RATE LIMIT")]
	pub rate_limit: Option<f64>,

	/// Duration of application to send requests. When duration is reached, application stops and exits. If duration is specified, n is ignored. Examples: -z 10s -z 3m
	#[arg(short = 'z', value_name = "Duration", value_parser = parse_duration)]
	pub max_duration: Option<Duration>,

	/// HTTP method, one of GET, POST, PUT, DELETE, HEAD, OPTIONS
	#[arg(short = 'm', value_parser = parse_method, default_value = "GET")]
	pub method: Method,

	/// Custom HTTP header. You can specify as many as needed by repeating the flag. For example, -H "Accept: text/html" -H "Content-Type: application/xml"
	#[arg(short = 'H', action = clap::ArgAction::Append)]
	pub headers: Vec<String>,

	/// Timeout for each request in seconds. Use 0 for infinite
	#[arg(short = 't', default_value = "20")]
	pub timeout: u64,

	/// HTTP Accept header
	#[arg(short = 'A', value_name = "ACCEPT HEADER", value_parser = parse_accept)]
	pub accept_header: Option<HeaderValue>,

	/// Content-type, defaults to "text/html"
	#[arg(short = 'T', value_name = "CONTENT-TYPE", default_value = "text/html", value_parser = parse_content_type)]
	pub content_type_header: HeaderValue,

	/// User-Agent, defaults to version "rey/0.1.0"
	#[arg(short = 'U', value_name = "USER AGENT", default_value = "rey/0.1.0", value_parser = parse_user_agent)]
	pub user_agent_header: HeaderValue,

	/// HTTP request body
	#[arg(short = 'd')]
	pub body: Option<String>,

	/// HTTP request body from file. For example, /home/user/file.txt or ./file.txt
	#[arg(short = 'D', value_name = "FILE")]
	pub body_file: Option<PathBuf>,

	/// Basic authentication, username:password
	#[arg(short = 'a', value_name = "USERNAME:PASSWORD", value_parser = parse_basic_auth)]
	pub basic_auth: Option<BasicAuth>,

	/// HTTP Proxy address as host:port
	#[arg(short = 'x', value_name = "PROXY")]
	pub proxy_address: Option<String>,

	#[arg(long = "host", value_name = "HOST")]
	pub host: Option<HeaderValue>,

	#[arg(
		long = "disable-redirects",
		value_name = "DISABLE REDIRECT",
		default_value = "false"
	)]
	pub disable_redirect: bool,
}

#[cfg(test)]
mod tests {
	use http::Method;

	use crate::arg::{
		parse_accept, parse_basic_auth, parse_content_type, parse_method, parse_user_agent,
	};
	use crate::work::BasicAuth;

	#[test]
	fn parse_method_should_work() {
		assert_eq!(Ok(Method::GET), parse_method("GET"));
		assert_eq!(Ok(Method::POST), parse_method("POST"));
		assert_eq!(Ok(Method::PUT), parse_method("PUT"));
		assert_eq!(Ok(Method::DELETE), parse_method("DELETE"));
		assert_eq!(Ok(Method::HEAD), parse_method("HEAD"));
		assert_eq!(Ok(Method::OPTIONS), parse_method("OPTIONS"));
		assert_eq!(Ok(Method::CONNECT), parse_method("CONNECT"));
		assert_eq!(Ok(Method::TRACE), parse_method("TRACE"));
		assert_eq!(Ok(Method::PATCH), parse_method("PATCH"));
	}

	#[test]
	fn parse_method_return_error() {
		assert_eq!(Err("invalid method"), parse_method(""));
		assert_eq!(Err("invalid method"), parse_method("HELLO"));
		assert_eq!(Err("invalid method"), parse_method("get"));
		assert_eq!(Err("invalid method"), parse_method("Get"));
		assert_eq!(Err("invalid method"), parse_method("大便"));
	}

	#[test]
	fn parse_url_should_work() {
		let url = parse_url("https://localhost:8080/api/v1/users?profile=true");
		let url = url.unwrap();
		assert_eq!("https", url.scheme());
		assert_eq!(Some("localhost"), url.host_str());
		assert_eq!(Some(8080_u16), url.port());
		assert_eq!("/api/v1/users", url.path());
		assert_eq!(Some("profile=true"), url.query());
	}

	#[test]
	fn parse_url_return_error() {
		assert_eq!(Err("invalid url"), parse_url(""));
		assert_eq!(Err("invalid url"), parse_url("not a url"));
		assert_eq!(Err("invalid url"), parse_url("不是url"));
		assert_eq!(Err("invalid url"), parse_url("127.0.0.1"));
	}

	#[test]
	fn parse_header_should_work() {
		assert_eq!(parse_accept("*/*").unwrap().to_str().unwrap(), "*/*");
		assert_eq!(
			parse_content_type("application/json")
				.unwrap()
				.to_str()
				.unwrap(),
			"application/json"
		);
		assert_eq!(parse_user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3").unwrap(),
                   "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3");
		assert_eq!(parse_host("rey-tools.io").unwrap(), "rey-tools.io");
	}

	#[test]
	fn parse_basic_auth_should_work() {
		assert_eq!(
			parse_basic_auth("root:123456"),
			Ok(BasicAuth {
				username: "root".to_string(),
				password: Some("123456".to_string())
			})
		);
		assert_eq!(
			parse_basic_auth("root"),
			Ok(BasicAuth {
				username: "root".to_string(),
				password: None
			})
		);
	}
}
