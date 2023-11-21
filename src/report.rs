use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tera::{to_value, try_get_value, Context, Filter, Tera, Value};

const BAR_CHAR: &str = "■";

const TEMPLATE: &str = r#"
Summary:
  Total:  {{ s.total | duration_to_sec_f64 | round(precision=4) }} secs
  Slowest:  {{ s.slowest | round(precision=4) }} secs
  Fastest:  {{ s.fastest | round(precision=4) }} secs
  Average:  {{ s.average | round(precision=4) }} secs
  Requests/sec:  {{ s.rps | round(precision=4) }}
  {% if s.size_total > 0 %}
  Total data:	{{ s.size_total | human_bytes }} bytes
  Size/request:	{{ s.size_req | human_bytes }} bytes {% endif %}

Response time histogram:
{{ s.histogram | histogram }}
Latency distribution: {% for dist in s.latency_dist %}
  {{ dist.percentage }}% in {{ dist.latency | round(precision=4) }} secs {% endfor %}

Status code distribution: {% for code, count in s.status_code_dist %}
  [{{ code }}]	{{ count }} responses{% endfor %}
{% if s.error_dist | length > 0 %}
Error distribution: {% for err, count in s.error_dist %}
  [{{ count }}] {{ err }}{% endfor %}{% endif %}
"#;

#[derive(Debug, Default, Serialize)]
pub struct LatencyDistribution {
	percentage: u8,
	latency: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Bucket {
	pub mark: f64,
	pub count: u64,
	pub frequency: f64,
}

struct DurationToSecF64Filter;

impl Filter for DurationToSecF64Filter {
	fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
		let duration = try_get_value!("duration_to_sec_f64", "value", Duration, value);
		Ok(to_value(duration.as_secs_f64())?)
	}
}

struct HumanBytesFilter;

impl Filter for HumanBytesFilter {
	fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
		let bytes = try_get_value!("human_bytes", "value", u64, value);
		Ok(to_value(human_bytes::human_bytes(bytes as f64))?)
	}
}

struct HistogramFilter;

impl Filter for HistogramFilter {
	fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
		let buckets = try_get_value!("histogram", "value", Vec<Bucket>, value);
		let max = buckets.iter().map(|bucket| bucket.count).max();
		let mut string = String::default();
		for ref bucket in buckets {
			let bar = max
				.map(|value| (bucket.count * 40 + value / 2) / value)
				.map(|len| BAR_CHAR.repeat(len as usize))
				.unwrap_or_default();
			string
				.push_str(format!("{:>4.3} [{}]\t|{}\n", bucket.mark, bucket.count, bar).as_str());
		}
		Ok(to_value(string)?)
	}
}

#[derive(Debug, Default, Serialize)]
pub struct Report {
	pub avg_total: f64,
	pub fastest: f64,
	pub slowest: f64,
	pub average: f64,
	pub rps: f64,

	pub total_requests: u64,

	pub total: Duration,

	pub error_dist: HashMap<String, u64>,
	pub status_code_dist: HashMap<u16, u64>,
	pub size_total: u64,
	pub size_req: u64,
	pub num_res: u64,

	pub latency_dist: Vec<LatencyDistribution>,
	pub histogram: Vec<Bucket>,
}

impl Report {
	pub fn print(&self) {
		let mut ctx = Context::new();
		ctx.insert("s", self);
		let mut tera = Tera::default();
		tera.register_filter("duration_to_sec_f64", DurationToSecF64Filter);
		tera.register_filter("human_bytes", HumanBytesFilter);
		tera.register_filter("histogram", HistogramFilter);
		let string = tera.render_str(TEMPLATE, &ctx).unwrap();
		println!("{}", string);
	}
}

#[derive(Default)]
pub struct Reporter {
	pub total_requests: u64,
	pub success_requests: u64,
	pub status_codes: Vec<u16>,
	pub size_total: u64,
	pub error_dist: HashMap<String, u64>,
	pub durations: Vec<f64>,
}

impl Reporter {
	fn histogram(&self, fastest: f64, slowest: f64) -> Vec<Bucket> {
		if self.success_requests == 0 {
			return vec![];
		}
		let bc = 10_usize;
		let mut buckets: Vec<f64> = Vec::with_capacity(bc + 1);
		let mut counts: Vec<u64> = vec![0; bc + 1];
		let bs = (slowest - fastest) / bc as f64;
		for i in 0..bc {
			buckets.push(fastest + bs * (i as f64));
		}
		buckets.push(slowest);

		let mut bi = 0_usize;
		let mut max = 0_u64;
		let mut i = 0_usize;
		while i < self.durations.len() {
			if self.durations[i] <= buckets[bi] {
				i += 1;
				counts[bi] += 1;
				if max < counts[bi] {
					max = counts[bi]
				}
			} else if bi < buckets.len() - 1 {
				bi += 1;
			}
		}

		buckets
			.iter()
			.zip(counts.iter())
			.map(|(bucket, count)| Bucket {
				mark: *bucket,
				count: *count,
				frequency: (*count) as f64 / self.durations.len() as f64,
			})
			.collect()
	}

	fn latencies(&self) -> Vec<LatencyDistribution> {
		let pctls = [10_u8, 25, 50, 75, 90, 95, 99];
		let mut data = vec![];
		let mut i = 0_usize;
		let mut j = 0_usize;
		while i < self.durations.len() && data.len() < pctls.len() {
			let current = i * 100 / self.durations.len();
			if current >= pctls[j] as usize {
				data.push(self.durations[i]);
				j += 1;
			}
			i += 1;
		}
		pctls
			.iter()
			.zip(data.iter())
			.map(|(p, d)| LatencyDistribution {
				percentage: *p,
				latency: *d,
			})
			.collect()
	}

	pub fn into_report(mut self, total: Duration) -> Report {
		let mut report = Report {
			total,
			rps: self.total_requests as f64 / total.as_secs_f64(),
			avg_total: self.durations.iter().sum(),
			total_requests: self.total_requests,
			size_total: self.size_total,
			..Report::default()
		};
		if self.success_requests > 0 {
			report.average = report.avg_total / self.success_requests as f64;
			report.size_req = self.size_total / self.success_requests;
		} else {
			report.average = 0.0;
			report.size_req = 0;
		}

		self.durations.sort_by(|a, b| a.total_cmp(b));
		report.fastest = *self.durations.first().unwrap_or(&0.0);
		report.slowest = *self.durations.last().unwrap_or(&0.0);
		report.histogram = self.histogram(report.fastest, report.slowest);
		report.latency_dist = self.latencies();
		report.error_dist = self.error_dist;
		report.status_code_dist =
			self.status_codes
				.into_iter()
				.fold(HashMap::new(), |mut map, code| {
					*map.entry(code).or_insert(0) += 1;
					map
				});

		report
	}
}
