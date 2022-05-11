use std::fs;
use std::{io::stdout, time::Duration};
use std::io::{Cursor, Stdout, Write, BufWriter};
use std::sync::Arc;

use anyhow::Result;

use headless_chrome::{Browser, Tab, protocol::cdp::Page::CaptureScreenshotFormatOption};
use sixel::{Environment, encoder};
use picto::{read, color::{Rgb, Srgb, Rgba}, Region};
use crossterm::{
    cursor,
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute, style, queue,
    terminal::{self, disable_raw_mode, enable_raw_mode, ClearType},
};
use termsize;

struct App {
    browser: Browser,
    tab: Arc<Tab>,
    stdout: Stdout,
    last_screen: String,
    limits: (u32, u32),
    scroll: (u32, u32),
}

impl App {
    fn new() -> Self {
        let browser = Browser::default().unwrap();
        let tab = browser.wait_for_initial_tab().unwrap();
        tab.navigate_to("https://ja.wikipedia.org/wiki/LISP")
            .unwrap()
            .wait_until_navigated()
            .unwrap();
        let stdout = stdout();
        let size = termsize::get().unwrap();
        Self {
            browser: browser,
            tab: tab,
            stdout: stdout,
            last_screen: String::new(),
            limits: (size.cols as u32, size.rows as u32),
            scroll: (0u32, 0u32)
        }
    }

    fn print_screen(&mut self) -> Result<()> {
        let html = self.tab.wait_for_element("html")?;
        html.call_js_fn(&format!("
            function(){{
                window.scroll({}, {});
            }}
        ", self.scroll.0, self.scroll.1), vec![], false)?;
        let png_data = self.tab
            .capture_screenshot(CaptureScreenshotFormatOption::Jpeg, Some(75),
            None,
            true)?;

        // let file = Cursor::new(jpeg_data);
        // let image = read::jpeg(file, |_| {}).unwrap();
        let image = read::from_memory::<Rgb, u8, _>(png_data).unwrap()
            .convert_with::<Rgba, u8, _>(|p: Rgb| Rgb::new(p.blue, p.red, p.green).into());
            // .convert_with::<Rgba, f32, _>(|p| Rgba::new(p.red, p.green, p.blue, 255).into())
        let mut settings = encoder::Settings::default();
        settings.high();
        settings.colors(256);
        // settings.size(self.limits);
        settings.size((800u32, 600u32));
        let mut out = Vec::new();
        {
            let mut stream = BufWriter::new(&mut out);
            encoder::encode(&settings, &image, &mut stream).unwrap();
        }
        self.last_screen = String::from_utf8(out).unwrap();

        Ok(())
    }

    fn poll_events(&mut self) -> Result<()> {
        loop {
            let event = read()?;

            if event == Event::Key(KeyCode::Esc.into()) {
                break;
            }

            else if event == Event::Key(KeyCode::Up.into()) {
                self.scroll.1 = self.scroll.1.saturating_sub(100u32);
                self.print_screen()?;
            }
            else if event == Event::Key(KeyCode::Down.into()) {
                self.scroll.1 += 100;
                self.print_screen()?;
            }

            if self.last_screen.is_empty() {
                self.print_screen()?;
            }

            queue!(
                self.stdout,
                style::ResetColor,
                // terminal::Clear(ClearType::All),
                cursor::Hide,
                cursor::MoveTo(1, 1),
                style::Print(&self.last_screen)
                )?;
            // queue!(self.stdout, style::Print(result.to_string()))?;
            // println!("{:?}", self.last_screen.unwrap());
            self.stdout.flush()?;

        }

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        execute!(self.stdout, terminal::EnterAlternateScreen)?;

        enable_raw_mode()?;

        // execute!(self.stdout, EnableMouseCapture)?;

        if let Err(e) = self.poll_events() {
            println!("Error: {:?}\r", e);
        }

        // execute!(self.stdout, DisableMouseCapture)?;

        execute!(self.stdout, style::ResetColor, cursor::Show, terminal::LeaveAlternateScreen)?;

        disable_raw_mode().unwrap();

        Ok(())
    }
}

fn main() -> Result<()> {
    let mut app = App::new();
    app.run();

    Ok(())
}
