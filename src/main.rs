use humansize::{format_size, DECIMAL};
use std::{env::args, path::Path, sync::Arc, time::Duration};
use tokio::{fs, io, sync::Mutex, time};

use colored::{Color, ColoredString, Colorize};
use waifu_pics::State;

use crate::waifu_pics::{waifu, Root};

mod waifu_pics;

fn lineup(count: usize) {
    println!("\x1b[{count}A")
}

fn indicator() -> ColoredString {
    "=".color(Color::Green)
}

fn fmt_state(state: &State) -> String {
    match state {
        State::Download { current, total } => {
            format!(
                "Downloading {} of {}",
                format_size(*current, DECIMAL).color(Color::Cyan),
                total
                    .map(|i| format_size(i, DECIMAL))
                    .unwrap_or("unknown".to_string())
                    .color(Color::Magenta)
            )
        }

        State::Failed => {
            format!("{}", "Failed".red())
        }

        State::FetchingURL => {
            format!("{}", "Fetching URL".color(Color::BrightGreen))
        }

        State::Done { target_file, .. } => {
            format!("{} {target_file}", "Wrote".green())
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut args = args();
    let mut can_say_goodbye = false;
    let total_to_dl = 8;
    args.next();

    let target = args.next().unwrap_or_else(|| String::from("waifus"));
    if !Path::new(&target).exists() {
        fs::create_dir_all(&target).await?;
    }

    println!("{} Waifu at my pc", indicator());
    println!("{} Output Directory: {}", indicator(), target);

    let root = Arc::new(Mutex::new(Root::default()));

    for _ in 0..total_to_dl {
        {
            let root = root.clone();

            tokio::spawn(waifu(root));
        }
    }

    println!("{} {}", indicator(), "State".yellow());
    loop {
        let mut current = root.lock().await;
        let mut to_remove = vec![];

        if can_say_goodbye {
            lineup(total_to_dl);
            println!("{} Downloaded OK", "=".repeat(10).cyan());
            return Ok(());
        }

        if current.tasks.len() <= 0 {
            continue;
        }

        let (w, _) = term_size::dimensions().unwrap();
        print!(
            "{}",
            ({
                let mut text = " ".repeat(w);
                text.push('\n');
                text
            })
            .repeat(total_to_dl)
        );

        lineup(total_to_dl);
        let tasks = current.tasks.clone();

        for (_id, task) in &tasks {
            println!(" {}: {}", task.name, fmt_state(&task.state));
        }

        for (id, task) in &tasks {
            if let State::Done { target_file, data } = &task.state {
                fs::write(format!("{target}/{target_file}"), data).await?;
                to_remove.push(id);
            }

            if let State::Failed = &task.state {}
        }

        for remove in to_remove {
            current.tasks.remove(&remove);

            if current.tasks.is_empty() {
                can_say_goodbye = true;
            }
        }

        println!();

        lineup(current.tasks.len() + 3);

        time::sleep(Duration::from_secs(1) / 60).await;
    }
}
