use yew::prelude::*;

pub mod status;

pub enum ModalMessage {
    Open(Html),
    CloseTop,
    CloseAll,
}

#[derive(Default, PartialEq)]
struct ModalState {
    modals: Vec<Html>,
}

impl Reducible for ModalState {
    type Action = ModalMessage;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        let mut modals = self.modals.clone();
        match action {
            ModalMessage::Open(h) => modals.push(h),
            ModalMessage::CloseTop => drop(modals.pop()),
            ModalMessage::CloseAll => modals.clear(),
        };
        Self { modals }.into()
    }
}

#[derive(Properties, PartialEq)]
pub struct ModalRendererProps {
    #[prop_or_default]
    pub children: Html,
}

pub type ModalRendererControls = Callback<ModalMessage, ()>;

#[function_component]
pub fn ModalRenderer(props: &ModalRendererProps) -> Html {
    let state = use_reducer(ModalState::default);
    let callback = {
        let state = state.clone();
        use_callback((), move |msg, ()| state.dispatch(msg) )
    };

    html! {
        <ContextProvider<ModalRendererControls> context={callback}>
            {props.children.clone()}
            <ModalContainers {state} />
        </ContextProvider<ModalRendererControls>>
    }
}

#[derive(Properties, PartialEq)]
struct ModalContainersProps {
    state: UseReducerHandle<ModalState>,
}

#[function_component]
fn ModalContainers(props: &ModalContainersProps) -> Html {
    let close_top = {
        let state = props.state.clone();
        use_callback((), move |_, ()| state.dispatch(ModalMessage::CloseTop))
    };

    html! {
        <>
            {for props.state.modals.iter().enumerate().map(|(i, modal)| {
                html! {
                    <div class="modal-container" style={format!("z-index: {};", i+1)} key={i}>
                        <div class="modal-background" onclick={close_top.clone()} />
                        <div class="modal-content">
                            {modal.clone()}
                        </div>
                    </div>
                }
            })}
        </>
    }
}
