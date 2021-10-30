/*! This file provides a way of constructing a webview based app. The idea is that the app developer provides an 'engine' that satisfies the [narthex_engine_trait] plus a simple main progrem, and the result is an app. See [narthex_engine_trait] for more information. See `narthex_wumpus` for an example of a main program that uses this crate. */
use ansi_term::Colour::*;
use anyhow::Result;
use log::{trace, error};
use narthex_engine_trait::{ActionTrait, EngineTrait, Event, ResponseKind, ResponseTrait};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::de::from_str;
use web_view::{escape, Content, WebView};
macro_rules! web_trace {
    () => { trace!() };
    ($($arg:tt)*) => {
        trace!("{} ({}:{})", Green.on(Black).paint(format!($($arg)*)), std::file!(), std::line!());
    };
}
macro_rules! web_error {
    () => { error!() };
    ($($arg:tt)*) => {
        error!("{} ({}:{})", Red.on(Black).paint(format!($($arg)*)), std::file!(), std::line!());
    };
}
/// parameters to running the engine
#[derive(Debug)]
pub struct WebParams {
    /// title for app
    pub title: String,
    /// whether to show web control messages
    pub debug: bool,
    /// height of window
    pub height: i32,
    /// width of window
    pub width: i32,
    /// Whether to show extra debug trace
    pub verbose: bool,
}
impl Default for WebParams {
    fn default() -> Self {
        Self {
            title: "App".to_string(),
            debug: false,
            width: 640,
            height: 960,
            verbose: false,
        }
    }
}
/** used by [web_view::WebView] */
pub struct UserData<Engine: EngineTrait> {
    engine: Engine,
}
impl<Engine> UserData<Engine>
where
    Engine: EngineTrait,
    Engine::Action: ActionTrait + DeserializeOwned + Sized + Clone,
    Engine::Response: ResponseTrait + Default + Serialize + std::fmt::Display,
{
    /// create
    pub fn new(engine: Engine) -> UserData<Engine> {
        UserData { engine }
    }
    /// build the web view and run the engine
    pub fn run_engine_with_webview(mut self, params: WebParams) -> Result<()> {
        web_trace!("running with engine, web view params are {:?}", &params);
        let initial_html = self.engine.initial_html()?;
        let webview: WebView<UserData<Engine>> = web_view::builder()
            .title(&params.title)
            .content(Content::Html(initial_html))
            .size(params.width, params.height)
            .resizable(true)
            .debug(params.debug)
            .user_data(self)
            .invoke_handler(|webview, arg: &str| {
                let action: Engine::Action = {
                    if params.verbose {
                        web_trace!("action: {}", &arg);
                    }
                    let action = from_str(&arg.to_owned()).unwrap_or_else(|e| {
                        web_error!("cannot deserialise: {:?}", &e);
                        panic!("cannot deserialise");
                    });
                    action
                };
                let response: Engine::Response = webview
                    .user_data_mut()
                    .engine
                    .execute(action)
                    .unwrap_or_else(|e| {
                        web_error!("bad execution: {:?}", &e);
                        Engine::Response::new_with_error(&format!("bad execution: {:?}", &e))
                    });

                if response.shutdown_required() {
                    // web_trace!("shutting down because response received: {}", &response);
                    if let ResponseKind::Error(msg) = response.kind() {
                        web_error!("system error: {}", msg);
                    }
                    webview.exit();
                } else {
                    let rs: String = serde_json::ser::to_string(&response).unwrap_or_else(|e| {
                        web_error!("cannot serialise: {:?}", &e);
                        panic!("cannot serialise");
                    });
                    //                    web_trace!(
                    //                        "response: {}",
                    //                        if rs.len() < 105 { &rs } else { &rs[..100] }
                    //                    );
                    let rsjs: String = escape(&rs).to_string();
                    //                    web_trace!(
                    //                        "resp to js: {}",
                    //                        if rsjs.len() < 105 {
                    //                            &rsjs
                    //                        } else {
                    //                            &rsjs[..100]
                    //                        }
                    //                    );
                    webview.eval(&format!("respond({});", &rsjs))?;
                }
                Ok(())
            })
            .build()?;
        let mut rres = webview.run()?;
        let _response = rres.engine.handle_event(&Event::Stop); // ignore the response
        Ok(())
    }
}
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */
