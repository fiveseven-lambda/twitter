#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let screen_name = "Re_menal2";

    match Client::from_config("config") {
        Ok(client) => {
            if let Some((last_id, last_time)) = {
                let response = {
                    let mut parameters = std::collections::BTreeMap::new();
                    parameters.insert("screen_name", screen_name);
                    parameters.insert("count", "1");
                    client.request(
                        reqwest::Method::GET,
                        "https://api.twitter.com/1.1/statuses/user_timeline.json",
                        &parameters
                    ).await?
                };
                let response_text = response.text().await?;
                let tweets = json::parse(&response_text).unwrap();
                if tweets.is_array() && tweets.len() > 0 {
                    let text = tweets[0]["text"].as_str().unwrap();
                    let user_name = tweets[0]["user"]["name"].as_str().unwrap();
                    let user_screen_name = tweets[0]["user"]["screen_name"].as_str().unwrap();
                    let id = tweets[0]["id"].as_u64().unwrap();
                    let created_at = tweets[0]["created_at"].as_str().unwrap();
                    println!("{}@{} {}\n#{}\n{}", user_name, user_screen_name, created_at, id, text);
                    Some((format!("{}", id), created_at.to_owned()))
                } else {
                    println!("{}", response_text);
                    None
                }
            } {
                println!("{} {}", last_id, last_time);
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let response = {
                        let mut parameters = std::collections::BTreeMap::new();
                        parameters.insert("screen_name", screen_name);
                        parameters.insert("count", "1");
                        parameters.insert("since_id", &last_id);
                        client.request(
                            reqwest::Method::GET,
                            "https://api.twitter.com/1.1/statuses/user_timeline.json",
                            &parameters
                        ).await?
                    };
                    let response_text = response.text().await?;
                    let tweets = json::parse(&response_text).unwrap();
                    if tweets.is_array() && tweets.len() > 0 {
                        let text = tweets[0]["text"].as_str().unwrap();
                        let user_name = tweets[0]["user"]["name"].as_str().unwrap();
                        let user_screen_name = tweets[0]["user"]["screen_name"].as_str().unwrap();
                        let id = tweets[0]["id"].as_u64().unwrap();
                        let created_at = tweets[0]["created_at"].as_str().unwrap();
                        println!("{}@{} {}\n#{}\n{}", user_name, user_screen_name, created_at, id, text);
                        let naive_last_time = chrono::DateTime::parse_from_str(&last_time, "%a %b %d %T %z %Y").unwrap().naive_utc();
                        let wake_time = if let Some((hour, min)) = re_menal_parse(text) {
                            chrono::Local::today().and_hms_opt(hour, min, 0)
                        } else {
                            None
                        };
                        let naive_wake_time = match wake_time {
                            Some(wake_time) => wake_time.naive_utc(),
                            None => chrono::DateTime::parse_from_str(&created_at, "%a %b %d %T %z %Y").unwrap().naive_utc()
                        };
                        let duration = naive_wake_time - naive_last_time;
                        let dhour = duration.num_hours();
                        let dmin = duration.num_minutes() - 60 * dhour;
                        let status = format!("@{} おはよう，{}！ {} 時間 {} 分寝てたんだね！", screen_name, user_name, dhour, dmin);
                        println!("{}", status);
                        let mut parameters = std::collections::BTreeMap::<&str, &str>::new();
                        let id_str = format!("{}", id);
                        parameters.insert("status", &status);
                        parameters.insert("in_reply_to_status_id", &id_str);
                        let response = client.request(
                            reqwest::Method::POST,
                            "https://api.twitter.com/1.1/statuses/update.json",
                            &parameters,
                        ).await?;
                        println!("{}", response.text().await?);
                        break;
                    }
                }
            }
        }
        Err(err) => {
            println!("failed to read config file: {}", err);
        }
    }
    Ok(())
}

struct Client {
    api_key: String,
    api_secret_key: String,
    access_token: String,
    access_token_secret: String,
}

impl Client {
    fn from_config(filename: &str) -> Result<Client, Box<dyn std::error::Error>> {
        let config = std::fs::File::open(filename)?;
        let mut reader = std::io::BufReader::new(config);
        fn read_line<T: std::io::BufRead>(
            reader: &mut T,
        ) -> Result<String, Box<dyn std::error::Error>> {
            let mut s = String::new();
            reader.read_line(&mut s)?;
            Ok(s.trim().to_owned())
        }
        Ok(Client {
            api_key: read_line(&mut reader)?,
            api_secret_key: read_line(&mut reader)?,
            access_token: read_line(&mut reader)?,
            access_token_secret: read_line(&mut reader)?,
        })
    }

    async fn request(
        &self,
        method: reqwest::Method,
        url: &str,
        parameters: &std::collections::BTreeMap<&str, &str>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let header_map = {
            use reqwest::header::*;
            let mut map = HeaderMap::new();
            map.insert(
                AUTHORIZATION,
                self.authorization(&method, url, parameters)
                    .parse()
                    .unwrap(),
            );
            map.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            );
            map
        };
        let url_with_parameters = format!(
            "{}?{}",
            url,
            equal_collect(parameters.iter().map(|(key, value)| { (*key, *value) })).join("&")
        );

        let client = reqwest::Client::new();
        client
            .request(method, &url_with_parameters)
            .headers(header_map)
            .send()
            .await
    }

    fn authorization(
        &self,
        method: &reqwest::Method,
        url: &str,
        parameters: &std::collections::BTreeMap<&str, &str>,
    ) -> String {
        let timestamp = format!("{}", chrono::Utc::now().timestamp());
        let nonce: String = {
            use rand::prelude::*;
            let mut rng = thread_rng();
            std::iter::repeat(())
                .map(|()| rng.sample(rand::distributions::Alphanumeric))
                .take(32)
                .collect()
        };

        let mut other_parameters: Vec<(&str, &str)> = vec![
            ("oauth_consumer_key", &self.api_key),
            ("oauth_token", &self.access_token),
            ("oauth_signature_method", "HMAC-SHA1"),
            ("oauth_version", "1.0"),
            ("oauth_timestamp", &timestamp),
            ("oauth_nonce", &nonce),
        ];

        let signature = self.signature(method, url, parameters.clone(), &other_parameters);

        other_parameters.push(("oauth_signature", &signature));

        format!(
            "OAuth {}",
            equal_collect(other_parameters.into_iter()).join(", ")
        )
    }

    fn signature<'a>(
        &self,
        method: &reqwest::Method,
        url: &str,
        mut parameters: std::collections::BTreeMap<&'a str, &'a str>,
        other_parameters: &Vec<(&'a str, &'a str)>,
    ) -> String {
        for (key, value) in other_parameters {
            parameters.insert(key, value);
        }
        let parameter_string = equal_collect(parameters.into_iter()).join("&");

        let signature_base_string = format!(
            "{}&{}&{}",
            method,
            percent_encode(url),
            percent_encode(&parameter_string)
        );
        let signing_key = format!("{}&{}", self.api_secret_key, self.access_token_secret);
        base64::encode(hmacsha1::hmac_sha1(
            signing_key.as_bytes(),
            signature_base_string.as_bytes(),
        ))
    }
}

fn equal_collect<'a, T: Iterator<Item = (&'a str, &'a str)>>(iter: T) -> Vec<String> {
    iter.map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect()
}

fn percent_encode(s: &str) -> percent_encoding::PercentEncode {
    use percent_encoding::*;
    const FRAGMENT: &AsciiSet = &NON_ALPHANUMERIC
        .remove(b'*')
        .remove(b'-')
        .remove(b'.')
        .remove(b'_');
    utf8_percent_encode(s, FRAGMENT)
}

fn re_menal_word(text: &str) -> Vec<(bool, &str)> {
    let mut vec = Vec::new();

    enum State {
        Space,
        Digit(usize),
        Word(usize)
    };

    let mut state = State::Space;
    
    for (i, c) in text.char_indices() {
        let next;
        if c.is_whitespace() {
            next = State::Space;
        } else if let Some(_) = c.to_digit(10) {
            next = State::Digit(i);
        } else {
            next = State::Word(i);
        }

        match state {
            State::Space => {
                state = next;
            }
            State::Digit(index) => {
                match next {
                    State::Digit(_) => {}
                    _ => {
                        vec.push((true, &text[index .. i]));
                        state = next;
                    }
                }
            }
            State::Word(index) => {
                vec.push((false, &text[index .. i]));
                state = next;
            }
        }
    }
    match state {
        State::Space => {}
        State::Digit(index) => {
            vec.push((true, &text[index..]));
        }
        State::Word(index) => {
            vec.push((false, &text[index..]));
        }
    }
    vec
}

fn re_menal_parse(text: &str) -> Option<(u32, u32)> {

    let words = re_menal_word(text);

    let mut vec = Vec::new();
    let mut half = false;
    let mut rev = false;

    for c in words {
        match c {
            (true, num) => {
                vec.push(num.parse().unwrap());
            }
            (false, word) => {
                if word == "半" {
                    half = true;
                } else if word == "前" {
                    rev = true;
                }
            }
        }
    }
    if vec.len() > 1 {
        if rev {
            Some((vec[0] - 1, 60 - vec[1]))
        } else {
            Some((vec[0], vec[1]))
        }
    } else if vec.len() == 1 {
        if half {
            Some((vec[0], 30))
        } else {
            Some((vec[0], 0))
        }
    } else {
        None
    }
}

#[test]
fn re_menal_parse_test() {
    assert_eq!(re_menal_parse("おはようございます！10時10分に起きました"), Some((10, 10)));
    assert_eq!(re_menal_parse("おはようございます！11時半に起きました"), Some((11, 30)));
    assert_eq!(re_menal_parse("おはようございます！11時に起きました"), Some((11, 0)));
    assert_eq!(re_menal_parse("おはようございます！11時の10分後くらいに起きました"), Some((11, 10)));
    assert_eq!(re_menal_parse("おはようございます！11時10分前に起きました"), Some((10, 50)));
    assert_eq!(re_menal_parse("おはようございます！11:10に起きました"), Some((11, 10)));
}
