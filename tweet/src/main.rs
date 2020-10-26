#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Client::from_config("config") {
        Ok(client) => {
            let mut parameters = std::collections::BTreeMap::new();

            // parameters.insert("status", "おはよう！");
            parameters.insert("count", "5");

            // let response = client.send(reqwest::Method::POST, "https://api.twitter.com/1.1/statuses/update.json", &parameters).await?;
            let response = client.send(reqwest::Method::GET, "https://api.twitter.com/1.1/statuses/home_timeline.json", &parameters).await?;
            let text = response.text().await?;
            println!("{}", text);
        }
        Err(err) => {
            println!("failed to read config file: {}", err);
        }
    }
    Ok(())
}

struct Client {
    api_key : String,
    api_secret_key : String,
    access_token : String,
    access_token_secret : String,
}

impl Client {
    fn from_config(filename: &str) -> Result<Client, Box<dyn std::error::Error>> {
        let config = std::fs::File::open(filename)?;
        let mut reader = std::io::BufReader::new(config);
        Ok(Client{
            api_key : read_line(&mut reader)?,
            api_secret_key : read_line(&mut reader)?,
            access_token : read_line(&mut reader)?,
            access_token_secret : read_line(&mut reader)?
        })
    }

    fn authorization(&self, method : &reqwest::Method, url : &str, parameters : &std::collections::BTreeMap<&str, &str>) -> String {
        let timestamp = format!("{}", chrono::Utc::now().timestamp());
        let nonce : String = {
            use rand::prelude::*;
            let mut rng = thread_rng();
            std::iter::repeat(())
                .map(|()| rng.sample(rand::distributions::Alphanumeric))
                .take(32)
                .collect()
        };

        let mut other_parameters : Vec::<(&str, &str)> = vec![
            ("oauth_consumer_key", &self.api_key),
            ("oauth_token", &self.access_token),
            ("oauth_signature_method", "HMAC-SHA1"),
            ("oauth_version", "1.0"),
            ("oauth_timestamp", &timestamp),
            ("oauth_nonce", &nonce)
        ];

        let parameter_string = {
            let mut all_parameters = parameters.clone();
            for (key, value) in &other_parameters {
                all_parameters.insert(key, value);
            }
            equal_collect(all_parameters.into_iter()).join("&")
        };

        let signature = self.signature(&parameter_string, &format!("{}", method), url);

        other_parameters.push(("oauth_signature", &signature));
        format!("OAuth {}", equal_collect(other_parameters.into_iter()).join(", "))
    }

    async fn send(&self, method: reqwest::Method, url: &str, parameters: &std::collections::BTreeMap<&str, &str>) -> Result<reqwest::Response, reqwest::Error> {
        let header_map = {
            use reqwest::header::*;
            let mut map = HeaderMap::new();
            map.insert(AUTHORIZATION, self.authorization(&method, url, parameters).parse().unwrap());
            map.insert(CONTENT_TYPE, HeaderValue::from_static("application/x-www-form-urlencoded"));
            map
        };

        let url_with_parameters = format!("{}?{}", url, equal_collect(parameters.iter().map(|(key, value)|{ (*key, *value) })).join("&"));

        let client = reqwest::Client::new();
        client.request(method, &url_with_parameters).headers(header_map).send().await
    }

    fn signature(&self, parameter_string: &str, method: &str, url: &str) -> String {
        let signature_base_string = format!("{}&{}&{}", method, percent_encode(url), percent_encode(&parameter_string));
        let signing_key = format!("{}&{}", self.api_secret_key, self.access_token_secret);
        base64::encode(hmacsha1::hmac_sha1(signing_key.as_bytes(), signature_base_string.as_bytes()))
    }
}

fn read_line(reader : &mut std::io::BufReader<std::fs::File>) -> Result<String, Box<dyn std::error::Error>> {
    let mut s = String::new();
    std::io::BufRead::read_line(reader, &mut s)?;
    Ok(s.trim().to_owned())
}

fn percent_encode(s : &str) -> percent_encoding::PercentEncode {
    use percent_encoding::*;
    const FRAGMENT: &AsciiSet = &NON_ALPHANUMERIC
        .remove(b'*')
        .remove(b'-')
        .remove(b'.')
        .remove(b'_');
    utf8_percent_encode(s, FRAGMENT)
}

fn equal_collect<'a, T : Iterator<Item = (&'a str, &'a str)>>(iter: T) -> Vec<String> {
    iter
        .map(|(key, value)|{
            format!("{}={}", percent_encode(key), percent_encode(value))
        })
        .collect()
}
