use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::ai::Categorizer;
use crate::db::Database;

/// Shared app state accessible from closures
struct AppState {
    db: Database,
    categorizer: Arc<Mutex<Option<Categorizer>>>,
    active_filter: RefCell<Option<String>>,
}

pub fn build_ui(app: &adw::Application) {
    // If window already exists (re-activation), just present it
    if let Some(win) = app.active_window() {
        win.present();
        return;
    }

    let db = Database::open().expect("Failed to open database");
    let categories = db.get_categories().unwrap_or_default();

    let categorizer: Arc<Mutex<Option<Categorizer>>> = Arc::new(Mutex::new(None));

    // Load AI model in background
    {
        let categorizer = categorizer.clone();
        let cats = categories.clone();
        std::thread::spawn(move || {
            match Categorizer::new(&cats) {
                Ok(c) => {
                    *categorizer.lock().unwrap() = Some(c);
                    eprintln!("AI model loaded successfully");
                }
                Err(e) => {
                    eprintln!("Failed to load AI model: {e}");
                }
            }
        });
    }

    let state = Rc::new(AppState {
        db,
        categorizer,
        active_filter: RefCell::new(None),
    });

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Thought Train")
        .default_width(700)
        .default_height(550)
        .build();

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    // Header bar
    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new("Thought Train", "capture → categorize → recall");
    header.set_title_widget(Some(&title));

    let search_btn = gtk::ToggleButton::new();
    search_btn.set_icon_name("system-search-symbolic");
    header.pack_start(&search_btn);

    main_box.append(&header);

    // Search bar
    let search_bar = gtk::SearchBar::new();
    let search_entry = gtk::SearchEntry::new();
    search_entry.set_hexpand(true);
    search_bar.set_child(Some(&search_entry));
    search_bar.connect_entry(&search_entry);
    search_btn
        .bind_property("active", &search_bar, "search-mode-enabled")
        .bidirectional()
        .build();
    main_box.append(&search_bar);

    // Content: sidebar + list
    let content_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    // --- Category sidebar ---
    let sidebar_scroll = gtk::ScrolledWindow::new();
    sidebar_scroll.set_width_request(170);
    sidebar_scroll.set_vexpand(true);
    sidebar_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

    let sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
    sidebar_box.add_css_class("navigation-sidebar");
    sidebar_box.set_margin_top(8);
    sidebar_box.set_margin_bottom(8);
    sidebar_box.set_margin_start(8);
    sidebar_box.set_margin_end(4);

    let sidebar_box_rc = Rc::new(sidebar_box);

    sidebar_scroll.set_child(Some(&*sidebar_box_rc));

    let sep = gtk::Separator::new(gtk::Orientation::Vertical);

    content_box.append(&sidebar_scroll);
    content_box.append(&sep);

    // --- Thought list area ---
    let list_area = gtk::Box::new(gtk::Orientation::Vertical, 8);
    list_area.set_hexpand(true);
    list_area.set_margin_top(12);
    list_area.set_margin_bottom(12);
    list_area.set_margin_start(12);
    list_area.set_margin_end(12);

    // Input row
    let input_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let input_entry = gtk::Entry::new();
    input_entry.set_hexpand(true);
    input_entry.set_placeholder_text(Some("What's on your mind?"));
    let add_btn = gtk::Button::with_label("Add");
    add_btn.add_css_class("suggested-action");
    input_box.append(&input_entry);
    input_box.append(&add_btn);
    list_area.append(&input_box);

    // Status label
    let status_label = gtk::Label::new(Some("Loading AI model..."));
    status_label.add_css_class("dim-label");
    status_label.set_halign(gtk::Align::Start);
    list_area.append(&status_label);

    // Thought list
    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_vexpand(true);
    let thought_list = gtk::ListBox::new();
    thought_list.add_css_class("boxed-list");
    thought_list.set_selection_mode(gtk::SelectionMode::None);
    scrolled.set_child(Some(&thought_list));
    list_area.append(&scrolled);

    content_box.append(&list_area);
    main_box.append(&content_box);
    window.set_content(Some(&main_box));

    // === Populate function (rebuilds both sidebar and thought list) ===
    let populate: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    {
        let state = state.clone();
        let thought_list = thought_list.clone();
        let sidebar_box_rc = sidebar_box_rc.clone();
        let populate_ref = populate.clone();
        let window = window.clone();

        let func = move || {
            // --- Rebuild sidebar ---
            // Remove all children
            while let Some(child) = sidebar_box_rc.first_child() {
                sidebar_box_rc.remove(&child);
            }

            let cats = state.db.get_categories().unwrap_or_default();

            // "All" button
            let all_btn = gtk::Button::with_label("All");
            all_btn.add_css_class("flat");
            {
                let state = state.clone();
                let populate_ref = populate_ref.clone();
                all_btn.connect_clicked(move |_| {
                    *state.active_filter.borrow_mut() = None;
                    if let Some(f) = populate_ref.borrow().as_ref() {
                        f();
                    }
                });
            }
            sidebar_box_rc.append(&all_btn);

            for cat in &cats {
                let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                row_box.set_hexpand(true);

                let btn = gtk::Button::with_label(cat);
                btn.add_css_class("flat");
                btn.set_hexpand(true);
                {
                    let state = state.clone();
                    let populate_ref = populate_ref.clone();
                    let cat = cat.clone();
                    btn.connect_clicked(move |_| {
                        *state.active_filter.borrow_mut() = Some(cat.clone());
                        if let Some(f) = populate_ref.borrow().as_ref() {
                            f();
                        }
                    });
                }
                row_box.append(&btn);

                // Delete category button (not for Misc — it's the fallback)
                if cat != "Misc" {
                    let del_btn = gtk::Button::from_icon_name("window-close-symbolic");
                    del_btn.add_css_class("flat");
                    del_btn.add_css_class("error");
                    del_btn.set_valign(gtk::Align::Center);
                    del_btn.set_tooltip_text(Some("Remove category"));
                    {
                        let state = state.clone();
                        let populate_ref = populate_ref.clone();
                        let cat = cat.clone();
                        del_btn.connect_clicked(move |_| {
                            if let Err(e) = state.db.delete_category(&cat) {
                                eprintln!("Delete category error: {e}");
                            }
                            // Reset filter if we were filtering by this category
                            let mut filter = state.active_filter.borrow_mut();
                            if filter.as_deref() == Some(&cat) {
                                *filter = None;
                            }
                            drop(filter);
                            if let Some(f) = populate_ref.borrow().as_ref() {
                                f();
                            }
                        });
                    }
                    row_box.append(&del_btn);
                }

                sidebar_box_rc.append(&row_box);
            }

            // "Add category" row
            let add_cat_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            add_cat_box.set_margin_top(8);
            let add_cat_entry = gtk::Entry::new();
            add_cat_entry.set_placeholder_text(Some("New category"));
            add_cat_entry.set_hexpand(true);
            let add_cat_btn = gtk::Button::from_icon_name("list-add-symbolic");
            add_cat_btn.add_css_class("flat");
            add_cat_btn.set_tooltip_text(Some("Add category"));

            let do_add_cat = {
                let state = state.clone();
                let populate_ref = populate_ref.clone();
                let add_cat_entry = add_cat_entry.clone();
                let categorizer = state.categorizer.clone();
                move || {
                    let name = add_cat_entry.text().trim().to_string();
                    if name.is_empty() {
                        return;
                    }
                    if let Err(e) = state.db.add_category(&name) {
                        eprintln!("Add category error: {e}");
                        return;
                    }
                    // Update AI model's category embeddings
                    if let Ok(mut guard) = categorizer.lock() {
                        if let Some(ref mut c) = *guard {
                            let cats = state.db.get_categories().unwrap_or_default();
                            let _ = c.update_categories(&cats);
                        }
                    }
                    add_cat_entry.set_text("");
                    if let Some(f) = populate_ref.borrow().as_ref() {
                        f();
                    }
                }
            };
            let do_add_cat = Rc::new(do_add_cat);

            {
                let do_add_cat = do_add_cat.clone();
                add_cat_btn.connect_clicked(move |_| (do_add_cat)());
            }
            {
                let do_add_cat = do_add_cat.clone();
                add_cat_entry.connect_activate(move |_| (do_add_cat)());
            }

            add_cat_box.append(&add_cat_entry);
            add_cat_box.append(&add_cat_btn);
            sidebar_box_rc.append(&add_cat_box);

            // --- Rebuild thought list ---
            while let Some(child) = thought_list.first_child() {
                thought_list.remove(&child);
            }

            let filter = state.active_filter.borrow().clone();
            let thoughts = match &filter {
                Some(cat) => state.db.get_thoughts(Some(cat)).unwrap_or_default(),
                None => state.db.get_thoughts(None).unwrap_or_default(),
            };

            let cats = state.db.get_categories().unwrap_or_default();

            for thought in &thoughts {
                let row = build_thought_row(thought, &state, &cats, &populate_ref, &window);
                thought_list.append(&row);
            }
        };

        *populate.borrow_mut() = Some(Box::new(func));
    }

    // Initial populate
    if let Some(f) = populate.borrow().as_ref() {
        f();
    }

    // --- AI status polling ---
    {
        let categorizer = state.categorizer.clone();
        let status_label = status_label.clone();
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            if categorizer.lock().unwrap().is_some() {
                status_label.set_text("AI ready");
                gtk::glib::ControlFlow::Break
            } else {
                gtk::glib::ControlFlow::Continue
            }
        });
    }

    // --- Add thought ---
    let add_thought = {
        let input_entry = input_entry.clone();
        let state = state.clone();
        let status_label = status_label.clone();
        let populate = populate.clone();
        move || {
            let text = input_entry.text().to_string();
            if text.trim().is_empty() {
                return;
            }

            let category = {
                let guard = state.categorizer.lock().unwrap();
                match &*guard {
                    Some(c) => c.categorize(&text).unwrap_or_else(|_| "Misc".to_string()),
                    None => "Misc".to_string(),
                }
            };

            status_label.set_text(&format!("Categorized as: {category}"));

            if let Err(e) = state.db.add_thought(&text, &category) {
                eprintln!("DB error: {e}");
                return;
            }

            input_entry.set_text("");
            if let Some(f) = populate.borrow().as_ref() {
                f();
            }
        }
    };

    let add_thought = Rc::new(add_thought);

    {
        let add_thought = add_thought.clone();
        add_btn.connect_clicked(move |_| (add_thought)());
    }
    {
        let add_thought = add_thought.clone();
        input_entry.connect_activate(move |_| (add_thought)());
    }

    // --- Search handler ---
    {
        let state = state.clone();
        let thought_list = thought_list.clone();
        let populate = populate.clone();
        let window = window.clone();
        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_string();
            while let Some(child) = thought_list.first_child() {
                thought_list.remove(&child);
            }
            let thoughts = if query.is_empty() {
                state.db.get_thoughts(None).unwrap_or_default()
            } else {
                state.db.search_thoughts(&query).unwrap_or_default()
            };
            let cats = state.db.get_categories().unwrap_or_default();
            for thought in &thoughts {
                let row = build_thought_row(thought, &state, &cats, &populate, &window);
                thought_list.append(&row);
            }
        });
    }

    // Focus the input entry on window present
    input_entry.grab_focus();
    window.present();
}

fn build_thought_row(
    thought: &crate::db::Thought,
    state: &Rc<AppState>,
    categories: &[String],
    populate: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    window: &adw::ApplicationWindow,
) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(&gtk::glib::markup_escape_text(&thought.text));
    row.set_subtitle(&format!("{} • {}", thought.category, thought.created_at));

    if thought.pinned {
        let pin_icon = gtk::Image::from_icon_name("starred-symbolic");
        row.add_prefix(&pin_icon);
    }

    let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    btn_box.set_valign(gtk::Align::Center);

    // Pin/unpin button
    let pin_btn = if thought.pinned {
        gtk::Button::from_icon_name("non-starred-symbolic")
    } else {
        gtk::Button::from_icon_name("starred-symbolic")
    };
    pin_btn.add_css_class("flat");
    pin_btn.set_tooltip_text(Some(if thought.pinned { "Unpin" } else { "Pin" }));
    {
        let state = state.clone();
        let populate = populate.clone();
        let id = thought.id;
        pin_btn.connect_clicked(move |_| {
            let _ = state.db.toggle_pin(id);
            if let Some(f) = populate.borrow().as_ref() {
                f();
            }
        });
    }
    btn_box.append(&pin_btn);

    // Edit button — opens a dialog to edit the text
    let edit_btn = gtk::Button::from_icon_name("document-edit-symbolic");
    edit_btn.add_css_class("flat");
    edit_btn.set_tooltip_text(Some("Edit"));
    {
        let state = state.clone();
        let populate = populate.clone();
        let id = thought.id;
        let current_text = thought.text.clone();
        let current_cat = thought.category.clone();
        let categories = categories.to_vec();
        let window = window.clone();
        edit_btn.connect_clicked(move |_| {
            show_edit_dialog(
                &window,
                &state,
                &populate,
                id,
                &current_text,
                &current_cat,
                &categories,
            );
        });
    }
    btn_box.append(&edit_btn);

    // Delete button
    let del_btn = gtk::Button::from_icon_name("user-trash-symbolic");
    del_btn.add_css_class("flat");
    del_btn.add_css_class("error");
    del_btn.set_tooltip_text(Some("Delete"));
    {
        let state = state.clone();
        let populate = populate.clone();
        let id = thought.id;
        del_btn.connect_clicked(move |_| {
            let _ = state.db.delete_thought(id);
            if let Some(f) = populate.borrow().as_ref() {
                f();
            }
        });
    }
    btn_box.append(&del_btn);

    row.add_suffix(&btn_box);
    row
}

fn show_edit_dialog(
    window: &adw::ApplicationWindow,
    state: &Rc<AppState>,
    populate: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    id: i64,
    current_text: &str,
    current_cat: &str,
    categories: &[String],
) {
    let dialog = adw::Window::builder()
        .transient_for(window)
        .modal(true)
        .title("Edit Thought")
        .default_width(450)
        .default_height(200)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let header = adw::HeaderBar::new();
    content.append(&header);

    let form = gtk::Box::new(gtk::Orientation::Vertical, 12);
    form.set_margin_top(16);
    form.set_margin_bottom(16);
    form.set_margin_start(16);
    form.set_margin_end(16);

    // Text entry
    let text_entry = gtk::Entry::new();
    text_entry.set_text(current_text);
    text_entry.set_placeholder_text(Some("Thought text"));
    form.append(&text_entry);

    // Category dropdown
    let cat_list: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();
    let cat_strings: gtk::StringList = gtk::StringList::new(&cat_list);
    let cat_dropdown = gtk::DropDown::new(Some(cat_strings), gtk::Expression::NONE);
    // Select current category
    if let Some(pos) = categories.iter().position(|c| c == current_cat) {
        cat_dropdown.set_selected(pos as u32);
    }
    let cat_label_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let cat_label = gtk::Label::new(Some("Category:"));
    cat_label_box.append(&cat_label);
    cat_label_box.append(&cat_dropdown);
    form.append(&cat_label_box);

    // Save button
    let save_btn = gtk::Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    save_btn.set_margin_top(8);
    {
        let state = state.clone();
        let populate = populate.clone();
        let dialog = dialog.clone();
        let text_entry = text_entry.clone();
        let cat_dropdown = cat_dropdown.clone();
        let categories = categories.to_vec();
        save_btn.connect_clicked(move |_| {
            let new_text = text_entry.text().to_string();
            let selected = cat_dropdown.selected() as usize;
            let new_cat = categories.get(selected).cloned().unwrap_or("Misc".to_string());

            if !new_text.trim().is_empty() {
                let _ = state.db.update_thought_text(id, new_text.trim());
                let _ = state.db.update_category(id, &new_cat);
            }
            dialog.close();
            if let Some(f) = populate.borrow().as_ref() {
                f();
            }
        });
    }
    form.append(&save_btn);

    // Enter key saves
    {
        let save_btn = save_btn.clone();
        text_entry.connect_activate(move |_| save_btn.emit_clicked());
    }

    content.append(&form);
    dialog.set_content(Some(&content));
    dialog.present();
}
