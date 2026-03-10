use iced::widget::{
    button, column, container, horizontal_space, pick_list, row, scrollable, svg, text,
    text_input, vertical_rule, Column,
};
use iced::{Border, Color, Element, Length, Padding, Task, Theme};
use std::sync::{Arc, Mutex};

use crate::ai::Categorizer;
use crate::db::{Database, Thought};

// Color palette - dark with blue/gray accents
const BG_DARK: Color = Color::from_rgb(0.09, 0.09, 0.12);
const BG_SURFACE: Color = Color::from_rgb(0.12, 0.13, 0.16);
const BG_CARD: Color = Color::from_rgb(0.15, 0.16, 0.20);
const BG_HOVER: Color = Color::from_rgb(0.18, 0.19, 0.24);
const BG_INPUT: Color = Color::from_rgb(0.11, 0.12, 0.15);
const ACCENT: Color = Color::from_rgb(0.35, 0.55, 0.85);
const ACCENT_DIM: Color = Color::from_rgb(0.25, 0.40, 0.65);
const ACCENT_GLOW: Color = Color::from_rgb(0.40, 0.62, 0.95);
const TEXT_PRIMARY: Color = Color::from_rgb(0.88, 0.90, 0.94);
const TEXT_SECONDARY: Color = Color::from_rgb(0.55, 0.58, 0.65);
const TEXT_DIM: Color = Color::from_rgb(0.40, 0.42, 0.48);
const DANGER: Color = Color::from_rgb(0.85, 0.30, 0.35);
const SUCCESS: Color = Color::from_rgb(0.30, 0.75, 0.45);
const WARNING: Color = Color::from_rgb(0.85, 0.75, 0.30);
const BORDER_SUBTLE: Color = Color::from_rgb(0.20, 0.22, 0.28);
const PINNED_GOLD: Color = Color::from_rgb(0.90, 0.78, 0.30);

const RADIUS_SM: f32 = 6.0;
const RADIUS_MD: f32 = 10.0;
const RADIUS_LG: f32 = 14.0;

const ICON_SM: f32 = 16.0;
const ICON_MD: f32 = 18.0;

// --- SVG icon handles (embedded at compile time) ---
fn icon_search() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../resources/icons/search.svg"))
}
fn icon_close() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../resources/icons/close.svg"))
}
fn icon_add() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../resources/icons/add.svg"))
}
fn icon_pin() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../resources/icons/pin.svg"))
}
fn icon_unpin() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../resources/icons/unpin.svg"))
}
fn icon_edit() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../resources/icons/edit.svg"))
}
fn icon_delete() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../resources/icons/delete.svg"))
}
fn icon_all() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../resources/icons/all.svg"))
}
fn icon_ai_ready() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../resources/icons/ai_ready.svg"))
}
fn icon_ai_loading() -> svg::Handle {
    svg::Handle::from_memory(include_bytes!("../resources/icons/ai_loading.svg"))
}

fn icon_for_category(category: &str) -> svg::Handle {
    let bytes: &[u8] = match category {
        "Work" => include_bytes!("../resources/icons/work.svg"),
        "Personal" => include_bytes!("../resources/icons/personal.svg"),
        "Ideas" => include_bytes!("../resources/icons/ideas.svg"),
        "Tasks" => include_bytes!("../resources/icons/tasks.svg"),
        "Health" => include_bytes!("../resources/icons/health.svg"),
        "Finance" => include_bytes!("../resources/icons/finance.svg"),
        "Learning" => include_bytes!("../resources/icons/learning.svg"),
        "Misc" => include_bytes!("../resources/icons/misc.svg"),
        _ => include_bytes!("../resources/icons/category.svg"),
    };
    svg::Handle::from_memory(bytes)
}

fn tinted_svg(handle: svg::Handle, color: Color, size: f32) -> iced::widget::Svg<'static, Theme> {
    svg(handle)
        .width(size)
        .height(size)
        .style(move |_: &Theme, _| svg::Style { color: Some(color) })
}

pub struct ThoughtTrain {
    db: Database,
    categorizer: Option<Categorizer>,
    ai_slot: Arc<Mutex<Option<Categorizer>>>,
    ai_status: AiStatus,

    thoughts: Vec<Thought>,
    categories: Vec<String>,
    active_filter: Option<String>,

    input_text: String,
    search_text: String,
    search_active: bool,
    new_category_text: String,
    status_message: String,

    editing: Option<EditState>,
}

struct EditState {
    thought_id: i64,
    text: String,
    selected_category: String,
}

#[derive(Debug, Clone)]
enum AiStatus {
    Loading,
    Ready,
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    AddThought,
    ToggleSearch,
    SearchChanged(String),
    SelectFilter(Option<String>),
    NewCategoryTextChanged(String),
    AddCategory,
    DeleteCategory(String),
    TogglePin(i64),
    DeleteThought(i64),
    StartEdit(i64, String, String),
    EditTextChanged(String),
    EditCategorySelected(String),
    SaveEdit,
    CancelEdit,
    AiModelLoaded(Result<(), String>),
}

impl ThoughtTrain {
    pub fn new() -> (Self, Task<Message>) {
        let db = Database::open().expect("Failed to open database");
        let categories = db.get_categories().unwrap_or_default();
        let thoughts = db.get_thoughts(None).unwrap_or_default();

        let ai_slot: Arc<Mutex<Option<Categorizer>>> = Arc::new(Mutex::new(None));
        let ai_slot_clone = ai_slot.clone();
        let cats_for_ai = categories.clone();

        let app = Self {
            db,
            categorizer: None,
            ai_slot,
            ai_status: AiStatus::Loading,
            thoughts,
            categories,
            active_filter: None,
            input_text: String::new(),
            search_text: String::new(),
            search_active: false,
            new_category_text: String::new(),
            status_message: "Loading AI model...".into(),
            editing: None,
        };

        let task = Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let c = Categorizer::new(&cats_for_ai).map_err(|e| e.to_string())?;
                    *ai_slot_clone.lock().unwrap() = Some(c);
                    Ok::<_, String>(())
                })
                .await
                .map_err(|e| e.to_string())?
            },
            Message::AiModelLoaded,
        );

        (app, task)
    }

    pub fn title(&self) -> String {
        "Thought Train".into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(val) => self.input_text = val,
            Message::AddThought => {
                let text = self.input_text.trim().to_string();
                if text.is_empty() {
                    return Task::none();
                }
                let category = match &self.categorizer {
                    Some(c) => c.categorize(&text).unwrap_or_else(|_| "Misc".into()),
                    None => "Misc".into(),
                };
                self.status_message = format!("Categorized as: {category}");
                let _ = self.db.add_thought(&text, &category);
                self.input_text.clear();
                self.refresh();
            }
            Message::ToggleSearch => {
                self.search_active = !self.search_active;
                if !self.search_active {
                    self.search_text.clear();
                    self.refresh();
                }
            }
            Message::SearchChanged(val) => {
                self.search_text = val;
                self.refresh();
            }
            Message::SelectFilter(filter) => {
                self.active_filter = filter;
                self.refresh();
            }
            Message::NewCategoryTextChanged(val) => self.new_category_text = val,
            Message::AddCategory => {
                let name = self.new_category_text.trim().to_string();
                if !name.is_empty() {
                    let _ = self.db.add_category(&name);
                    if let Some(ref mut c) = self.categorizer {
                        let cats = self.db.get_categories().unwrap_or_default();
                        let _ = c.update_categories(&cats);
                    }
                    self.new_category_text.clear();
                    self.refresh();
                }
            }
            Message::DeleteCategory(name) => {
                let _ = self.db.delete_category(&name);
                if self.active_filter.as_deref() == Some(&name) {
                    self.active_filter = None;
                }
                self.refresh();
            }
            Message::TogglePin(id) => {
                let _ = self.db.toggle_pin(id);
                self.refresh();
            }
            Message::DeleteThought(id) => {
                let _ = self.db.delete_thought(id);
                self.refresh();
            }
            Message::StartEdit(id, text, category) => {
                self.editing = Some(EditState {
                    thought_id: id,
                    text,
                    selected_category: category,
                });
            }
            Message::EditTextChanged(val) => {
                if let Some(ref mut e) = self.editing {
                    e.text = val;
                }
            }
            Message::EditCategorySelected(cat) => {
                if let Some(ref mut e) = self.editing {
                    e.selected_category = cat;
                }
            }
            Message::SaveEdit => {
                if let Some(edit) = self.editing.take() {
                    let t = edit.text.trim().to_string();
                    if !t.is_empty() {
                        let _ = self.db.update_thought_text(edit.thought_id, &t);
                        let _ = self.db.update_category(edit.thought_id, &edit.selected_category);
                    }
                    self.refresh();
                }
            }
            Message::CancelEdit => self.editing = None,
            Message::AiModelLoaded(result) => match result {
                Ok(()) => {
                    self.categorizer = self.ai_slot.lock().unwrap().take();
                    self.ai_status = AiStatus::Ready;
                    self.status_message = "AI ready".into();
                }
                Err(e) => {
                    self.ai_status = AiStatus::Failed(e.clone());
                    self.status_message = format!("AI failed: {e}");
                }
            },
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = self.view_header();
        let sidebar = self.view_sidebar();
        let main_content = self.view_main_content();

        let body = row![
            container(sidebar).width(200).style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(BG_SURFACE)),
                ..Default::default()
            }),
            vertical_rule(1),
            main_content,
        ]
        .height(Length::Fill);

        let mut page = column![header];
        if self.search_active {
            page = page.push(self.view_search_bar());
        }
        page = page.push(body);

        let page = container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(BG_DARK)),
                ..Default::default()
            });

        if let Some(ref edit) = self.editing {
            iced::widget::stack![
                page,
                container(self.view_edit_overlay(edit))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(|_: &Theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            0.0, 0.0, 0.0, 0.7,
                        ))),
                        ..Default::default()
                    }),
            ]
            .into()
        } else {
            page.into()
        }
    }

    fn view_header(&self) -> Element<'_, Message> {
        let title = text("Thought Train")
            .size(22)
            .color(ACCENT_GLOW);
        let subtitle = text("talk to me")
            .size(11)
            .color(TEXT_DIM);

        let search_icon = if self.search_active {
            tinted_svg(icon_close(), TEXT_SECONDARY, ICON_MD)
        } else {
            tinted_svg(icon_search(), TEXT_SECONDARY, ICON_MD)
        };
        let search_btn = button(search_icon)
            .on_press(Message::ToggleSearch)
            .padding([6, 12])
            .style(btn_ghost);

        container(
            row![
                column![title, subtitle].spacing(2),
                horizontal_space(),
                search_btn,
            ]
            .padding(Padding::from([14, 20]))
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .style(|_: &Theme| container::Style {
            background: Some(iced::Background::Color(BG_SURFACE)),
            border: Border {
                width: 0.0,
                radius: 0.0.into(),
                color: BORDER_SUBTLE,
            },
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
    }

    fn view_search_bar(&self) -> Element<'_, Message> {
        container(
            container(
                row![
                    tinted_svg(icon_search(), TEXT_DIM, ICON_SM),
                    text_input("Search thoughts...", &self.search_text)
                        .on_input(Message::SearchChanged)
                        .padding(10)
                        .size(14)
                        .style(input_style),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .padding(Padding::from([8, 20])),
        )
        .style(|_: &Theme| container::Style {
            background: Some(iced::Background::Color(BG_SURFACE)),
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let mut sidebar = Column::new().spacing(2).padding(Padding::from([12, 10]));

        let section_label = text("CATEGORIES")
            .size(10)
            .color(TEXT_DIM);
        sidebar = sidebar.push(container(section_label).padding(Padding::from([4, 8])));
        sidebar = sidebar.push(iced::widget::Space::with_height(4));

        // "All" button
        let is_all = self.active_filter.is_none();
        let all_color = if is_all { ACCENT_GLOW } else { TEXT_SECONDARY };
        sidebar = sidebar.push(
            sidebar_btn_with_icon(icon_all(), all_color, "All", is_all, Message::SelectFilter(None)),
        );

        sidebar = sidebar.push(iced::widget::Space::with_height(2));

        for cat in &self.categories {
            let is_active = self.active_filter.as_deref() == Some(cat);
            let color = if is_active { ACCENT_GLOW } else { TEXT_SECONDARY };

            let cat_btn = sidebar_btn_with_icon(
                icon_for_category(cat),
                color,
                cat,
                is_active,
                Message::SelectFilter(Some(cat.clone())),
            );

            if cat != "Misc" {
                let del = button(tinted_svg(icon_close(), TEXT_DIM, 12.0))
                    .on_press(Message::DeleteCategory(cat.clone()))
                    .padding([4, 6])
                    .style(btn_ghost);
                sidebar = sidebar.push(
                    row![cat_btn.width(Length::Fill), del]
                        .align_y(iced::Alignment::Center),
                );
            } else {
                sidebar = sidebar.push(cat_btn.width(Length::Fill));
            }
        }

        sidebar = sidebar.push(iced::widget::Space::with_height(8));
        let sep_label = text("ADD NEW")
            .size(10)
            .color(TEXT_DIM);
        sidebar = sidebar.push(container(sep_label).padding(Padding::from([4, 8])));
        sidebar = sidebar.push(iced::widget::Space::with_height(4));

        let add_input = text_input("Category name...", &self.new_category_text)
            .on_input(Message::NewCategoryTextChanged)
            .on_submit(Message::AddCategory)
            .padding(8)
            .size(12)
            .style(input_style);

        let add_btn = button(tinted_svg(icon_add(), ACCENT, ICON_SM))
            .on_press(Message::AddCategory)
            .padding([6, 10])
            .style(btn_ghost);

        sidebar = sidebar.push(
            row![add_input, add_btn]
                .spacing(4)
                .align_y(iced::Alignment::Center),
        );

        scrollable(sidebar).height(Length::Fill).into()
    }

    fn view_main_content(&self) -> Element<'_, Message> {
        let mut content = Column::new()
            .spacing(12)
            .padding(Padding::from([16, 20]))
            .width(Length::Fill);

        // Input area
        let input = text_input("What's on your mind?", &self.input_text)
            .on_input(Message::InputChanged)
            .on_submit(Message::AddThought)
            .padding(12)
            .size(15)
            .style(input_style);

        let add_btn = button(
            row![
                tinted_svg(icon_add(), Color::WHITE, ICON_SM),
                text("Add").size(13).color(Color::WHITE),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::AddThought)
        .padding([10, 20])
        .style(btn_accent);

        content = content.push(
            row![input, add_btn]
                .spacing(10)
                .align_y(iced::Alignment::Center),
        );

        // Status indicator
        let (status_color, status_icon) = match &self.ai_status {
            AiStatus::Ready => (SUCCESS, icon_ai_ready()),
            AiStatus::Loading => (WARNING, icon_ai_loading()),
            AiStatus::Failed(_) => (DANGER, icon_close()),
        };
        content = content.push(
            row![
                tinted_svg(status_icon, status_color, 12.0),
                text(&self.status_message).size(11).color(status_color),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        );

        // Thought count
        let count = self.thoughts.len();
        let filter_label = match &self.active_filter {
            Some(cat) => format!("{cat} ({count})"),
            None => format!("All thoughts ({count})"),
        };
        content = content.push(text(filter_label).size(12).color(TEXT_DIM));

        // Thought list
        let mut list = Column::new().spacing(8);
        for thought in &self.thoughts {
            list = list.push(self.view_thought_row(thought));
        }
        if self.thoughts.is_empty() {
            list = list.push(
                container(
                    column![
                        text("\u{2014}").size(28).color(TEXT_DIM),
                        text("No thoughts here yet").size(14).color(TEXT_SECONDARY),
                        text("Type something above to get started")
                            .size(12)
                            .color(TEXT_DIM),
                    ]
                    .spacing(8)
                    .align_x(iced::Alignment::Center),
                )
                .padding(40)
                .center_x(Length::Fill),
            );
        }

        content = content.push(scrollable(list).height(Length::Fill));
        content.into()
    }

    fn view_thought_row<'a>(&'a self, thought: &'a Thought) -> Element<'a, Message> {
        let pin_indicator: Element<'a, Message> = if thought.pinned {
            tinted_svg(icon_pin(), PINNED_GOLD, ICON_SM).into()
        } else {
            iced::widget::Space::with_width(ICON_SM).into()
        };

        let title = text(&thought.text).size(15).color(TEXT_PRIMARY);

        let cat_svg = tinted_svg(icon_for_category(&thought.category), TEXT_SECONDARY, 12.0);
        let subtitle_text =
            text(format!("{} \u{00B7} {}", thought.category, thought.created_at))
                .size(11)
                .color(TEXT_SECONDARY);
        let subtitle = row![cat_svg, subtitle_text]
            .spacing(4)
            .align_y(iced::Alignment::Center);

        let pin_handle = if thought.pinned {
            icon_unpin()
        } else {
            icon_pin()
        };
        let pin_btn = button(tinted_svg(pin_handle, PINNED_GOLD, ICON_SM))
            .on_press(Message::TogglePin(thought.id))
            .padding([6, 10])
            .style(btn_ghost);

        let edit_btn = button(tinted_svg(icon_edit(), ACCENT, ICON_SM))
            .on_press(Message::StartEdit(
                thought.id,
                thought.text.clone(),
                thought.category.clone(),
            ))
            .padding([6, 10])
            .style(btn_ghost);

        let del_btn = button(tinted_svg(icon_delete(), DANGER, ICON_SM))
            .on_press(Message::DeleteThought(thought.id))
            .padding([6, 10])
            .style(btn_ghost);

        let left = row![pin_indicator, column![title, subtitle].spacing(3)]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .width(Length::Fill);

        let actions = row![pin_btn, edit_btn, del_btn]
            .spacing(2)
            .align_y(iced::Alignment::Center);

        let border_left_color = if thought.pinned { PINNED_GOLD } else { ACCENT_DIM };

        container(
            row![left, actions]
                .spacing(8)
                .align_y(iced::Alignment::Center),
        )
        .padding(Padding::from([12, 14]))
        .style(move |_: &Theme| container::Style {
            background: Some(iced::Background::Color(BG_CARD)),
            border: Border {
                radius: RADIUS_SM.into(),
                width: 0.0,
                color: border_left_color,
            },
            ..Default::default()
        })
        .width(Length::Fill)
        .into()
    }

    fn view_edit_overlay(&self, edit: &EditState) -> Element<'_, Message> {
        let title_row = row![
            tinted_svg(icon_edit(), ACCENT_GLOW, ICON_MD),
            text("Edit Thought").size(18).color(ACCENT_GLOW),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let text_field = text_input("Thought text...", &edit.text)
            .on_input(Message::EditTextChanged)
            .on_submit(Message::SaveEdit)
            .padding(12)
            .size(14)
            .style(input_style);

        let cat_label = text("Category").size(12).color(TEXT_SECONDARY);
        let cat_picker = pick_list(
            self.categories.clone(),
            Some(edit.selected_category.clone()),
            Message::EditCategorySelected,
        )
        .padding(10);

        let save_btn = button(
            row![
                text("\u{2713}").size(14).color(Color::WHITE),
                text("Save").size(13).color(Color::WHITE),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::SaveEdit)
        .padding([10, 24])
        .style(btn_accent);

        let cancel_btn = button(
            row![
                tinted_svg(icon_close(), TEXT_SECONDARY, 14.0),
                text("Cancel").size(13).color(TEXT_SECONDARY),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::CancelEdit)
        .padding([10, 24])
        .style(btn_ghost);

        let form = column![
            title_row,
            iced::widget::Space::with_height(4),
            text_field,
            column![cat_label, cat_picker].spacing(4),
            iced::widget::Space::with_height(4),
            row![save_btn, cancel_btn].spacing(10),
        ]
        .spacing(14)
        .width(440);

        container(
            container(form.padding(24)).style(|_: &Theme| container::Style {
                background: Some(iced::Background::Color(BG_SURFACE)),
                border: Border {
                    radius: RADIUS_LG.into(),
                    width: 1.0,
                    color: BORDER_SUBTLE,
                },
                ..Default::default()
            }),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn refresh(&mut self) {
        self.categories = self.db.get_categories().unwrap_or_default();
        self.thoughts = if self.search_active && !self.search_text.is_empty() {
            self.db.search_thoughts(&self.search_text).unwrap_or_default()
        } else {
            self.db.get_thoughts(self.active_filter.as_deref()).unwrap_or_default()
        };
    }
}

// --- Style helpers ---

fn sidebar_btn_with_icon<'a>(
    handle: svg::Handle,
    color: Color,
    label: &'a str,
    active: bool,
    msg: Message,
) -> button::Button<'a, Message> {
    let icon = tinted_svg(handle, color, ICON_SM);
    button(
        row![icon, text(label).size(14).color(color)]
            .spacing(8)
            .align_y(iced::Alignment::Center),
    )
    .on_press(msg)
    .padding([8, 12])
    .width(Length::Fill)
    .style(move |_: &Theme, status| {
        let bg = match status {
            button::Status::Hovered => BG_HOVER,
            _ if active => Color { a: 0.12, ..ACCENT },
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: color,
            border: Border {
                radius: RADIUS_SM.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
}

fn btn_accent(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => ACCENT_GLOW,
        button::Status::Pressed => ACCENT_DIM,
        _ => ACCENT,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: RADIUS_MD.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn btn_ghost(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => BG_HOVER,
        button::Status::Pressed => BG_CARD,
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: TEXT_SECONDARY,
        border: Border {
            radius: RADIUS_SM.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn input_style(_: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused => ACCENT,
        text_input::Status::Hovered => ACCENT_DIM,
        _ => BORDER_SUBTLE,
    };
    text_input::Style {
        background: iced::Background::Color(BG_INPUT),
        border: Border {
            radius: RADIUS_SM.into(),
            width: 1.0,
            color: border_color,
        },
        icon: TEXT_DIM,
        placeholder: TEXT_DIM,
        value: TEXT_PRIMARY,
        selection: Color { a: 0.3, ..ACCENT },
    }
}
