use crate::crypto::{Candle, Granularity};
use chrono::DateTime;
use itertools::Itertools;
use rocket::response::content::RawHtml;
use std::path::PathBuf;
use ta::{indicators::ExponentialMovingAverage, Next};

pub struct Analyzer {
    database: PathBuf,
}

impl Analyzer {
    pub fn new(database: PathBuf) -> Self {
        Self { database }
    }

    pub fn analyze(
        &self,
        crypto: &str,
        granularity: Granularity,
        num_candlesticks: usize,
    ) -> anyhow::Result<RawHtml<String>> {
        println!("analyzing {crypto} - {granularity:?}");
        let conn = sqlite::open(&self.database)?;
        let query = format!(
            "SELECT DISTINCT open, high, low, close, volume, time FROM candles WHERE crypto='{crypto}' AND granularity='{}' ORDER BY time DESC LIMIT {num_candlesticks}",
            <&'static str>::from(granularity)
        );
        println!("{query}");
        let mut statement = conn.prepare(query)?;

        let mut candles = vec![];
        while let Ok(sqlite::State::Row) = statement.next() {
            candles.push(Candle {
                open: statement.read::<f64, _>("open").unwrap(),
                high: statement.read::<f64, _>("high").unwrap(),
                low: statement.read::<f64, _>("low").unwrap(),
                close: statement.read::<f64, _>("close").unwrap(),
                volume: statement.read::<f64, _>("volume").unwrap(),
                time: DateTime::from_timestamp(statement.read::<i64, _>("time").unwrap(), 0)
                    .unwrap(),
            });
        }
        println!("num candles: {}", candles.len());

        Ok(RawHtml(candlestick_chart_html(crypto, &candles)))
    }
}

fn candlestick_data(candlesticks: &[Candle]) -> String {
    candlesticks
        .iter()
        .map(
            |Candle {
                 time,
                 low,
                 high,
                 open,
                 close,
                 volume: _,
             }| {
                let date = time.to_rfc3339();
                format!("{{x: new Date('{date}'), y:[{open}, {high}, {low}, {close}]}}")
            },
        )
        .join(",")
}

fn crypto_ema(candlesticks: &[Candle]) -> String {
    let mut ema = ExponentialMovingAverage::new(2).unwrap();
    candlesticks
        .iter()
        .map(|Candle { time, close, .. }| {
            let date = time.to_rfc3339();
            let value = ema.next(*close);
            format!("{{x: new Date('{date}'), y:{value}}}")
        })
        .join(",")
}

fn candlestick_chart_html(crypto: &str, candlesticks: &[Candle]) -> String {
    let candlestick_data = candlestick_data(candlesticks);
    let ema_data = crypto_ema(candlesticks);

    format!(
        r#"
    <html>
    <head>
    <script type="text/javascript">
    window.onload = function () {{
        var chart = new CanvasJS.Chart("chartContainer",
        {{
            zoomEnabled: true,
            title:{{
                text: "{crypto} Candlestick Chart",
                fontFamily: "times new roman"
            }},
            zoomEnabled: true,
            exportEnabled: true,
            axisY: {{
                includeZero:false,
                title: "Prices",
                prefix: "$ "
            }},
            axisX: {{
                labelAngle: -45
            }},
            data: [
            {{
                type: "candlestick",
                dataPoints: [{candlestick_data}]
            }},
            {{
                type: "line",
                name: "EMA",
                color: "orange",
                dataPoints: [{ema_data}]
            }}
            ]
        }});
        chart.render();
    }}
    </script>
    <script type="text/javascript" src="https://cdn.canvasjs.com/canvasjs.min.js"></script>
    </head>
    <body>
    <div id="chartContainer" style="height: 100%; width: 100%;">
    </div>
    </body>
    </html>
    "#
    )
}
