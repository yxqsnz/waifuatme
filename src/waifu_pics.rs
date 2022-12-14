use futures_util::StreamExt;
use reqwest as http;
use reqwest::Error;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
const BASE: &str = "https://api.waifu.pics/sfw";
type HttpResult<T> = Result<T, Error>;

#[derive(Debug, Clone)]
pub enum State {
    FetchingURL,
    Failed,
    Download { current: usize, total: Option<u64> },
    Done { target_file: String, data: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: usize,
    pub name: String,
    pub state: State,
}

#[derive(Default)]
pub struct Root {
    pub tasks: HashMap<usize, Task>,
    pub id: usize,
}

impl Root {
    pub fn new_task(&mut self, name: impl ToString) -> usize {
        self.id += 1;
        self.tasks.insert(
            self.id,
            Task {
                id: self.id,
                name: name.to_string(),
                state: State::FetchingURL,
            },
        );
        self.id
    }
}

async fn set_state_by_id(root: &Arc<Mutex<Root>>, id: usize, state: State) -> Option<()> {
    let mut root = root.lock().await;
    root.tasks.get_mut(&id)?.state = state;
    Some(())
}

pub async fn waifu(root: Arc<Mutex<Root>>) -> HttpResult<()> {
    let id = root.lock().await.new_task("Waifu.Pic");

    let json = http::get(format!("{BASE}/waifu"))
        .await?
        .json::<HashMap<String, String>>()
        .await?;

    if let Some(url) = json.get("url") {
        let res = http::get(url).await?;
        let len = res.content_length();
        let mut stream = res.bytes_stream();
        let mut total_wrote = 0;
        let mut content = vec![];

        while let Some(Ok(chunk)) = stream.next().await {
            total_wrote += chunk.len();
            content.append(&mut chunk.to_vec());

            set_state_by_id(
                &root,
                id,
                State::Download {
                    current: total_wrote,
                    total: len,
                },
            )
            .await
            .unwrap();
        }
        let splices = url.split("/");
        let last = splices.last().unwrap_or("unknown.file");

        set_state_by_id(
            &root,
            id,
            State::Done {
                target_file: last.to_string(),
                data: content,
            },
        )
        .await
        .unwrap();
    } else {
        set_state_by_id(&root, id, State::Failed).await.unwrap();
    }

    Ok(())
}
