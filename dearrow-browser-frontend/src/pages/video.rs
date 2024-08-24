/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2023-2024 mini_bomba
*  
*  This program is free software: you can redistribute it and/or modify
*  it under the terms of the GNU Affero General Public License as published by
*  the Free Software Foundation, either version 3 of the License, or
*  (at your option) any later version.
*
*  This program is distributed in the hope that it will be useful,
*  but WITHOUT ANY WARRANTY; without even the implied warranty of
*  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*  GNU Affero General Public License for more details.
*
*  You should have received a copy of the GNU Affero General Public License
*  along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use std::rc::Rc;

use error_handling::{anyhow, ErrContext, ErrorContext, ResContext};
use dearrow_browser_api::unsync::{InnertubeVideo, Video};
use gloo_console::error;
use yew::prelude::*;
use yew_hooks::{use_async_with_options, UseAsyncHandle, UseAsyncOptions};
use yew_router::prelude::Link;

use crate::components::detail_table::*;
use crate::components::icon::{Icon, IconType};
use crate::components::youtube::{OriginalTitle, YoutubeIframe};
use crate::contexts::WindowContext;
use crate::hooks::{use_async_suspension, use_location_state};
use crate::innertube::{self, youtu_be_link};
use crate::pages::MainRoute;
use crate::thumbnails::components::{Thumbnail, ThumbnailCaption};
use crate::utils::{api_request, sbb_video_link, RcEq};

#[derive(Properties, PartialEq)]
struct VideoDetailsTableProps {
    videoid: AttrValue,
    mode: DetailType,
    metadata: UseAsyncHandle<Rc<Video>, RcEq<ErrorContext>>,
}

#[function_component]
fn VideoDetailsTable(props: &VideoDetailsTableProps) -> Html {
    let youtube_url: Rc<AttrValue> = use_memo(props.videoid.clone(), |vid| AttrValue::Rc(youtu_be_link(vid).as_str().into()));
    let sbb_url: Rc<AttrValue> = use_memo(props.videoid.clone(), |vid| AttrValue::Rc(sbb_video_link(vid).as_str().into()));
    let fallback = html!{
        <span><em>{"Loading..."}</em></span>
    };
    html! {
        <div class="info-table">
            <div>{format!("Video ID: {}", props.videoid)}</div>
            <div hidden={props.mode != DetailType::Title}>
                {"Original title: "}
                <Suspense {fallback}><OriginalTitle videoid={props.videoid.clone()} /></Suspense>
            </div>
            if props.metadata.loading {
                <div><em>{"Loading extra metadata..."}</em></div>
            } else if let Some(ref data) = props.metadata.data {
                if let Some(duration) = data.duration {
                    if props.mode == DetailType::Thumbnail {
                        <div>{format!("Random thumbnail timestamp: {}", duration*data.random_thumbnail)}</div>
                    }
                    <div>{format!("Video duration: {duration}")}</div>
                } else {
                    if props.mode == DetailType::Thumbnail {
                        <div>{format!("Random thumbnail: {}%", data.random_thumbnail*100.)}</div>
                    }
                    <div>{"Video duration: "}<em>{"Unknown"}</em></div>
                }
                <div title="This is the fraction of the video that has not been covered by any live SponsorBlock skip segments. Sections marked by SponsorBlock are excluded from possible random thumbnail timestamp picks">
                    {format!("% of video unmarked: {}%", data.fraction_unmarked*100.)}
                </div>
                <div title="If there is no marked outro, the last 10% of the video is assumed to be an outro and is excluded from possible random thumbnail timestamp picks">
                    {"Has a marked outro: "}
                    if data.has_outro {
                        {"Yes"}
                    } else {
                        {"No"}
                    }
                </div>
            } else {
                <div><em>{"Failed to fetch extra metadata."}</em></div>
            } 
            
            <div><a href={&*youtube_url}>{"View on YouTube"}</a></div>
            <div><a href={&*sbb_url}>{"View on SB Browser"}</a></div>
            <Suspense><ChannelLink videoid={props.videoid.clone()} /></Suspense>
        </div>
    }
}

#[function_component]
fn ChannelLink(props: &VideoPageProps) -> HtmlResult {
    let channel_handle = use_async_suspension(|vid| async move {
        let result = innertube::get_oembed_info(&vid).await;
        if let Err(ref e) = result {
            error!(format!("Failed to fetch oembed info for video {vid}: {e:?}"));
        }
        let result = result?;
        let url = match reqwest::Url::parse(&result.author_url) {
            Err(e) => {
                error!(format!("Failed to parse channel url for video {vid}: {e:?}"));
                return Err(e.context("Failed to parse channel URL"))
            },
            Ok(u) => u,
        };
        match url.path_segments().and_then(|ps|
            ps.filter(|s| !s.is_empty()).last()
        ) {
            Some(handle) => Ok(AttrValue::from(handle.to_owned())),
            None => {
                error!(format!("Failed to extract channel handle from url for video {vid}!"));
                Err(anyhow!("Failed to extract channel handle"))
            }
        }
    }, props.videoid.clone())?;

    let Ok(ref channel_handle) = *channel_handle else { return Ok(html! {}) };
    let route = MainRoute::Channel { id: channel_handle.clone() };
    
    Ok(html! {
        <Link<MainRoute> to={route}>{"Browse this channel's page "}<Icon r#type={IconType::DABLogo} /></Link<MainRoute>>
    })
}

#[derive(Properties, PartialEq)]
pub struct VideoPageProps {
    pub videoid: AttrValue,
}

#[function_component]
pub fn VideoPage(props: &VideoPageProps) -> Html {
    let window_context: Rc<WindowContext> = use_context().expect("WindowContext should be defined");
    let state = use_location_state().get_state();
    let entry_count = use_state_eq(|| None);

    let metadata: UseAsyncHandle<Rc<Video>, RcEq<ErrorContext>> = {
        let video_id = props.videoid.clone();
        let window_context = window_context.clone();
        use_async_with_options(async move {
            async move {
                let dab_api_url = window_context.origin_join_segments(&["api", "videos", &video_id]);
                let mut video: Video = api_request(dab_api_url).await.context("Metadata request failed")?;

                if video.duration.is_none() {
                    let it_duration: Result<u64, ErrorContext> = async move {
                        let it_dab_url = window_context.origin_join_segments(&["innertube", "video", &video_id]);
                        let it_video: InnertubeVideo = api_request(it_dab_url).await.context("Proxied innertube request failed")?;
                        Ok(it_video.duration)
                    }.await;
                    match it_duration {
                        Err(e) => error!(format!("Failed to fetch video duration from innertube: {e:?}")),
                        #[allow(clippy::cast_precision_loss)]
                        Ok(d) => video.duration = Some(d as f64),
                    }
                }
                
                Ok(video)
            }.await.map(Rc::new).map_err(RcEq::new)
        }, UseAsyncOptions::enable_auto())
    };

    let api_url = use_memo((state.detail_table_mode, props.videoid.clone()), |(dtm, vid)|{
        match dtm {
            DetailType::Title => window_context.origin_join_segments(&["api", "titles", "video_id", vid]),
            DetailType::Thumbnail => window_context.origin_join_segments(&["api", "thumbnails", "video_id", vid]),
        }
    });

    let rc_videoid = use_memo(props.videoid.clone(), |videoid| {
        match videoid {
            AttrValue::Rc(ref rc) => rc.clone(),
            AttrValue::Static(s) => (*s).into(),
        }
    });

    let fallback = html! {
        <center><b>{"Loading..."}</b></center>
    };
    
    html! {
        <>
            <div class="page-details">
                <YoutubeIframe videoid={props.videoid.clone()} />
                if state.detail_table_mode == DetailType::Thumbnail {
                    <Thumbnail video_id={(*rc_videoid).clone()} timestamp={None} caption={ThumbnailCaption::Text("Original thumbnail".into())} />
                    if let Some(ref metadata) = metadata.data {
                        if let Some(duration) = metadata.duration {
                            <Thumbnail video_id={(*rc_videoid).clone()} timestamp={Some(duration*metadata.random_thumbnail)} caption={ThumbnailCaption::Text("Random thumbnail".into())} />
                        }
                    }
                }
                <VideoDetailsTable videoid={props.videoid.clone()} mode={state.detail_table_mode} {metadata} />
            </div>
            <TableModeSwitch entry_count={*entry_count} />
            <Suspense {fallback}>
                <PaginatedDetailTableRenderer mode={state.detail_table_mode} url={api_url} {entry_count} hide_videoid=true />
            </Suspense>
        </>
    }
}
