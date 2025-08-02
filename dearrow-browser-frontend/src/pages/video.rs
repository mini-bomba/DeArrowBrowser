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

use cloneable_errors::{ErrorContext, ResContext};
use dearrow_browser_api::unsync::{ApiThumbnail, ApiTitle, InnertubeVideo, Video};
use gloo_console::error;
use reqwest::Url;
use strum::{IntoStaticStr, VariantArray};
use yew::prelude::*;
use yew_router::prelude::{Location, LocationHandle, RouterScopeExt};

use crate::components::tables::remote::{Endpoint, RemotePaginatedTable};
use crate::components::tables::switch::TableModeSwitch;
use crate::components::tables::thumbs::ThumbTableSettings;
use crate::components::tables::titles::TitleTableSettings;
use crate::components::youtube::{ChannelLink, OriginalTitle, YoutubeIframe};
use crate::contexts::WindowContext;
use crate::innertube::youtu_be_link;
use crate::pages::{MainRoute, LocationState};
use crate::thumbnails::components::{Thumbnail, ThumbnailCaption};
use crate::utils::{api_request, sbb_video_link, RcEq, ReqwestUrlExt, SimpleLoadState};

#[derive(Properties, PartialEq)]
struct VideoDetailsTableProps {
    videoid: AttrValue,
    tab: VideoPageTab,
    metadata: MetadataState,
}

#[function_component]
fn VideoDetailsTable(props: &VideoDetailsTableProps) -> Html {
    let youtube_url: Rc<AttrValue> = use_memo(props.videoid.clone(), |vid| {
        AttrValue::Rc(youtu_be_link(vid).as_str().into())
    });
    let sbb_url: Rc<AttrValue> = use_memo(props.videoid.clone(), |vid| {
        AttrValue::Rc(sbb_video_link(vid).as_str().into())
    });
    html! {
        <div class="info-table">
            <div>{format!("Video ID: {}", props.videoid)}</div>
            <div>
                {"Channel: "}
                <ChannelLink videoid={props.videoid.clone()} />
            </div>
            <div hidden={props.tab != VideoPageTab::Titles}>
                {"Original title: "}
                <OriginalTitle videoid={props.videoid.clone()} />
            </div>
            {match &props.metadata {
                MetadataState::Loading => html! {<div><em>{"Loading extra metadata..."}</em></div>},
                MetadataState::Failed => html! {<div><em>{"Failed to fetch extra metadata."}</em></div>},
                MetadataState::Ready(data) => html! {<>
                    if let Some(duration) = data.duration {
                        if props.tab == VideoPageTab::Thumbnails {
                            <div>{format!("Random thumbnail timestamp: {}", duration*data.random_thumbnail)}</div>
                        }
                        <div>{format!("Video duration: {duration}")}</div>
                    } else {
                        if props.tab == VideoPageTab::Thumbnails {
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
                </>},
            }}

            <div><a href={&*youtube_url}>{"View on YouTube"}</a></div>
            <div><a href={&*sbb_url}>{"View on SB Browser"}</a></div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct VideoPageProps {
    pub videoid: AttrValue,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, VariantArray, IntoStaticStr)]
enum VideoPageTab {
    #[default]
    Titles,
    Thumbnails,
}

#[derive(PartialEq, Eq, Clone)]
struct VideoPageTitles {
    videoid: AttrValue,
}
#[derive(PartialEq, Eq, Clone)]
struct VideoPageThumbnails {
    videoid: AttrValue,
}

impl Endpoint for VideoPageTitles {
    type Item = ApiTitle;
    type LoadProgress = ();

    fn create_url(&self, base_url: &Url) -> Url {
        base_url
            .join_segments(&["api", "titles", "video_id", &self.videoid])
            .expect("base_url should be a valid base")
    }
}
impl Endpoint for VideoPageThumbnails {
    type Item = ApiThumbnail;
    type LoadProgress = ();

    fn create_url(&self, base_url: &Url) -> Url {
        base_url
            .join_segments(&["api", "thumbnails", "video_id", &self.videoid])
            .expect("base_url should be a valid base")
    }
}

pub type MetadataState = SimpleLoadState<RcEq<Video>>;

pub struct VideoPage {
    tab: VideoPageTab,
    origin: Url,
    rc_videoid: Rc<str>,

    entry_count: Option<usize>,
    entry_count_callback: Callback<Option<usize>>,
    metadata: MetadataState,
    version: u8,

    _location_listener: LocationHandle,
    _wc_listener: ContextHandle<Rc<WindowContext>>,
}

impl VideoPage {
    fn refresh(&mut self, ctx: &Context<Self>) {
        self.version = self.version.wrapping_add(1);
        self.metadata = MetadataState::Loading;
        let version = self.version;
        let video_id = ctx.props().videoid.clone();
        let dab_api_url = self
            .origin
            .join_segments(&["api", "videos", &video_id])
            .expect("origin should be a base");
        let it_dab_url = self
            .origin
            .join_segments(&["innertube", "video", &video_id])
            .expect("origin should be a base");
        ctx.link().send_future(async move {
            let result: Result<_, ErrorContext> = async {
                let mut video: Video = api_request(dab_api_url)
                    .await
                    .context("Metadata request failed")?;

                if video.duration.is_none() {
                    let it_duration: Result<u64, ErrorContext> = async move {
                        let it_video: InnertubeVideo = api_request(it_dab_url)
                            .await
                            .context("Proxied innertube request failed")?;
                        Ok(it_video.duration)
                    }
                    .await;
                    match it_duration {
                        Err(e) => error!(format!(
                            "Failed to fetch video duration from innertube: {e:?}"
                        )),
                        #[allow(clippy::cast_precision_loss)]
                        Ok(d) => video.duration = Some(d as f64),
                    }
                }
                Ok(RcEq::from(video))
            }
            .await;
            let result = match result {
                Ok(meta) => MetadataState::Ready(meta),
                Err(err) => {
                    error!(format!(
                        "Failed to fetch extra metadata for video {video_id}: {err:?}"
                    ));
                    MetadataState::Failed
                }
            };
            VideoPageMessage::MetadataFetched {
                data: result,
                version,
            }
        });
    }
}

pub enum VideoPageMessage {
    LocationUpdated(Location),
    OriginUpdated(Url),
    EntryCountUpdate(Option<usize>),
    MetadataFetched { data: MetadataState, version: u8 },
}

impl Component for VideoPage {
    type Properties = VideoPageProps;
    type Message = VideoPageMessage;

    fn create(ctx: &Context<Self>) -> Self {
        let scope = ctx.link();

        let state = match scope
            .location()
            .unwrap()
            .state::<LocationState<VideoPageTab>>()
        {
            Some(state) => *state,
            None => {
                let state = LocationState::default();
                scope
                    .navigator()
                    .unwrap()
                    .replace_with_state(&scope.route::<MainRoute>().unwrap(), state);
                state
            }
        };

        let (wc, wc_listener) = scope
            .context(scope.callback(|wc: Rc<WindowContext>| {
                VideoPageMessage::OriginUpdated(wc.origin.clone())
            }))
            .expect("WindowContext should be available");

        let mut this = Self {
            tab: state.detail_table_mode,
            origin: wc.origin.clone(),
            rc_videoid: match ctx.props().videoid {
                AttrValue::Rc(ref rc) => rc.clone(),
                AttrValue::Static(s) => s.into(),
            },

            entry_count: None,
            entry_count_callback: scope.callback(VideoPageMessage::EntryCountUpdate),
            metadata: MetadataState::Loading,
            version: 0,

            _location_listener: scope
                .add_location_listener(scope.callback(VideoPageMessage::LocationUpdated))
                .unwrap(),
            _wc_listener: wc_listener,
        };
        this.refresh(ctx);
        this
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        const TITLE_SETTINGS: TitleTableSettings = TitleTableSettings {
            hide_videoid: true,
            hide_userid: false,
            hide_username: false,
        };
        const THUMB_SETTINGS: ThumbTableSettings = ThumbTableSettings {
            hide_videoid: true,
            hide_userid: false,
            hide_username: false,
        };

        let props = ctx.props();
        html! {<>
            <div class="page-details">
                <YoutubeIframe videoid={props.videoid.clone()} />
                if self.tab == VideoPageTab::Thumbnails {
                    <Thumbnail
                        video_id={self.rc_videoid.clone()}
                        timestamp={None}
                        caption={ThumbnailCaption::Text("Original thumbnail".into())}
                    />
                    if let MetadataState::Ready(metadata) = &self.metadata {
                        if let Video {duration: Some(duration), random_thumbnail, ..} = **metadata {
                            <Thumbnail
                                video_id={self.rc_videoid.clone()}
                                timestamp={Some(duration*random_thumbnail)}
                                caption={ThumbnailCaption::Text("Random thumbnail".into())}
                            />
                        }
                    }
                }
                <VideoDetailsTable videoid={props.videoid.clone()} tab={self.tab} metadata={self.metadata.clone()} />
            </div>
            <TableModeSwitch<VideoPageTab> entry_count={self.entry_count} />
            {match self.tab {
                VideoPageTab::Titles => html! {
                    <RemotePaginatedTable<VideoPageTitles, VideoPageTab>
                        endpoint={VideoPageTitles {
                            videoid: props.videoid.clone()
                        }}
                        item_count_update={self.entry_count_callback.clone()}
                        settings={TITLE_SETTINGS}
                    />
                },
                VideoPageTab::Thumbnails => html! {
                    <RemotePaginatedTable<VideoPageThumbnails, VideoPageTab>
                        endpoint={VideoPageThumbnails {
                            videoid: props.videoid.clone()
                        }}
                        item_count_update={self.entry_count_callback.clone()}
                        settings={THUMB_SETTINGS}
                    />
                },
            }}
        </>}
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            VideoPageMessage::OriginUpdated(origin) => {
                if self.origin == origin {
                    false
                } else {
                    self.origin = origin;
                    self.refresh(ctx);
                    true
                }
            }
            VideoPageMessage::LocationUpdated(location) => {
                let scope = ctx.link();
                let state = match location
                    .state::<LocationState<VideoPageTab>>()
                    .or_else(|| scope.location().unwrap().state())
                {
                    Some(state) => *state,
                    None => {
                        let state = LocationState::default();
                        scope
                            .navigator()
                            .unwrap()
                            .replace_with_state(&scope.route::<MainRoute>().unwrap(), state);
                        state
                    }
                };
                if self.tab == state.detail_table_mode {
                    false
                } else {
                    self.tab = state.detail_table_mode;
                    true
                }
            }
            VideoPageMessage::EntryCountUpdate(entry_count) => {
                if self.entry_count == entry_count {
                    false
                } else {
                    self.entry_count = entry_count;
                    true
                }
            }
            VideoPageMessage::MetadataFetched { data, version } => {
                if self.version == version {
                    self.metadata = data;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if ctx.props().videoid == old_props.videoid {
            false
        } else {
            self.rc_videoid = match ctx.props().videoid {
                AttrValue::Rc(ref rc) => rc.clone(),
                AttrValue::Static(str) => str.into(),
            };
            self.refresh(ctx);
            true
        }
    }
}
