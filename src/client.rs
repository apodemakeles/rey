use std::time::Duration;

use anyhow::{anyhow, Context, Error, Result};
use http::header::{ACCEPT, CONTENT_TYPE, HOST, USER_AGENT};
use http::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{Client, Proxy};

pub struct ClientBuilder {
	pub headers: Vec<String>,
	pub timeout: Option<Duration>,
	pub content_type: HeaderValue,
	pub accept: Option<HeaderValue>,
	pub user_agent: HeaderValue,
	pub proxy: Option<String>,
	pub host: Option<HeaderValue>,
	pub disable_redirect: bool,
}

impl TryFrom<ClientBuilder> for Client {
	type Error = anyhow::Error;

	fn try_from(cb: ClientBuilder) -> anyhow::Result<Client> {
		let mut headers = try_into_headers(&cb.headers)?;
		headers.insert(CONTENT_TYPE, cb.content_type);
		headers.insert(USER_AGENT, cb.user_agent);
		if let Some(accept) = cb.accept {
			headers.insert(ACCEPT, accept);
		}
		if let Some(host) = cb.host {
			headers.insert(HOST, host);
		}

		let mut builder = Client::builder();
		builder = builder.default_headers(headers);

		if let Some(timeout) = cb.timeout {
			builder = builder.timeout(timeout).connect_timeout(timeout);
		}
		if cb.disable_redirect {
			builder = builder.redirect(Policy::none())
		}
		if let Some(proxy) = cb.proxy {
			builder = builder.proxy(Proxy::all(proxy).context("invalid proxy")?);
		}
		builder.build().context("fail to build a http client")
	}
}

type Header = (HeaderName, HeaderValue);

fn try_into_headers(strs: &[String]) -> Result<HeaderMap, Error> {
	strs.iter()
		.map(|s| try_into_header(s))
		.collect::<Result<Vec<Header>, Error>>()
		.map(|headers| {
			headers
				.into_iter()
				.fold(HeaderMap::new(), |mut map, (name, value)| {
					map.insert(name, value);
					map
				})
		})
}

fn try_into_header(s: &str) -> Result<Header> {
	let parts: Vec<&str> = s.splitn(2, ':').collect();
	let get_error = || anyhow!("{} is not a valid header", s);
	if parts.len() != 2 {
		return Err(anyhow!("{} is not a valid header", s));
	}
	let name = HeaderName::try_from(parts[0].trim()).map_err(|_| get_error())?;
	let value = HeaderValue::try_from(parts[1].trim()).map_err(|_| get_error())?;
	Ok((name, value))
}

#[cfg(test)]
mod test {
	use crate::client::{try_into_header, try_into_headers};

	#[test]
	fn try_into_header_should_work() {
		let (name, value) = try_into_header("Token: abcdefg").unwrap();
		assert_eq!(name.as_str(), "token");
		assert_eq!(value.to_str().unwrap(), "abcdefg");
		let (name, value) = try_into_header("Accept-Language: gzip, deflate").unwrap();
		assert_eq!(name.as_str(), "accept-language");
		assert_eq!(value.to_str().unwrap(), "gzip, deflate");
	}

	#[test]
	fn try_into_headers_should_work() {
		let vec = vec![
			"Token: abcdefg".to_string(),
			"Accept-Language: gzip, deflate".to_string(),
		];
		let headers = try_into_headers(&vec).unwrap();
		assert_eq!(headers.get("Token").unwrap().to_str().unwrap(), "abcdefg");
		assert_eq!(
			headers.get("Accept-Language").unwrap().to_str().unwrap(),
			"gzip, deflate"
		);
	}
}
