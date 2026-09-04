// irealstudio — a11y smoke-прототип на Rust + wxDragon.
//
// Цель: проверить, что настоящее нативное меню wxWidgets (Win32 HMENU)
// читается NVDA как меню: Alt открывает менюбар, стрелки ходят по пунктам,
// NVDA объявляет пункты и их состояние, Enter активирует, Esc закрывает.
// Это ровно те свойства, которых не хватило egui (там меню было «кнопками»).
//
// Второй слой проверки: нативные контролы — список песен (wxListBox) и
// читаемый текст чарта (wxTextCtrl, readonly multiline) — без WebView2,
// поэтому переключения деревьев (IA2 Chromium <-> Win32/UIA) нет и фриза
// NVDA, как в Go/Glaze-гибриде, быть не должно.
//
// Сборка на Windows — из "x64 Native Tools Command Prompt for VS 2022":
//   cargo run
// Первая сборка ДОЛГАЯ: wxdragon-sys качает исходники wxWidgets 3.3.3 и
// собирает их статически (нужны C++-тулчейн + CMake + Ninja). Это нормально.
// Консоль не прячем: если что-то не поднимется, ошибка будет видна в stdout.

use wxdragon::prelude::*;

/// Псевдо-библиотека стандартов — те же песни, что в egui-прототипе,
/// чтобы сравнение «egui vs wx» было честным (одинаковый контент).
const SONG_TITLES: &[&str] = &[
    "Rhythm Changes",
    "All The Things You Are",
    "Autumn Leaves",
    "Blue Bossa",
    "Giant Steps",
    "Take Five",
];

/// Образец чарта (та же нотация, что в egui-прототипе). Символы аккордов —
/// нотация, не текст для озвучки, поэтому не «прописью».
const CHART: &str = r#"Rhythm Changes — AABA

[A1]
F^7  F^7  | Bb7  Bb7 | F^7  F^7  | Gm7  C7
F^7  A7   | Dm7  Db7 | Cm7  F7   | Bb7  C7

[A2]
F^7  F^7  | Bb7  Bb7 | F^7  F^7  | Gm7  C7
A7   D7   | Gm7  C7  | F7   D7   | Gm7  C7

[B]
C7   C7   | F7   F7  | C7   C7   | A7   D7
G7   G7   | C7   C7  | F7   F7   | F7   A7

[A3]
Dm7  G7   | Gm7  C7  | F^7  A7   | Dm7  Db7
Cm7  F7   | Bb7  A7  | Dm7  G7   | Gm7  C7  F^7"#;

// --- ID пунктов меню (свои константы; ID_EXIT/ID_ABOUT берём из прелюда) ---
const ID_OPEN: i32 = 1001;
const ID_SAVE: i32 = 1002;
const ID_COPY_LOCAL: i32 = 1003;
const ID_PASTE_LOCAL: i32 = 1004;
const ID_SPEAK: i32 = 3001;
const ID_RESET: i32 = 3002;

fn main() {
    // wx без манифеста показывает на старте предупреждение про common controls —
    // глушим проверку (манифест добавим в релизной сборке через build.rs).
    SystemOptions::set_option_by_int("msw.no-manifest-check", 1);

    let _ = wxdragon::main(|_app| {
        // --- Главное окно ---
        let frame = Frame::builder()
            .with_title("irealstudio — прототип wxDragon (нативное меню)")
            .with_size(Size::new(920, 640))
            .build();

        // --- Менюбар: Файл / Правка / Вид / Справка ---
        let file_menu = Menu::builder()
            .append_item(ID_OPEN, "&Открыть…\tCtrl+O", "Открыть песню")
            .append_item(ID_SAVE, "&Сохранить…\tCtrl+S", "Сохранить изменения")
            .append_separator()
            .append_item(ID_EXIT, "&Выход", "Закрыть программу")
            .build();

        let edit_menu = Menu::builder()
            .append_item(ID_COPY_LOCAL, "Копировать", "Скопировать выделенное")
            .append_item(ID_PASTE_LOCAL, "Вставить", "Вставить из буфера")
            .build();

        let view_menu = Menu::builder()
            .append_item(ID_SPEAK, "Озвучить чарт", "Прочитать чарт вслух")
            .append_item(ID_RESET, "Сбросить статус", "Вернуть статус-строку к Готово")
            .build();

        let help_menu = Menu::builder()
            .append_item(ID_ABOUT, "О прототипе", "Информация о сборке")
            .build();

        let menu_bar = MenuBar::builder()
            .append(file_menu, "&Файл")
            .append(edit_menu, "&Правка")
            .append(view_menu, "&Вид")
            .append(help_menu, "&Справка")
            .build();
        frame.set_menu_bar(menu_bar);

        // --- Статусная строка (build сам прикрепляет её к frame) ---
        StatusBar::builder(&frame)
            .with_fields_count(1)
            .add_initial_text(0, "Готов. Alt — меню, стрелки — список песен, Esc — закрыть меню.")
            .build();

        // --- Левая панель: библиотека ---
        let lib_label = StaticText::builder(&frame)
            .with_label("Библиотека")
            .build();
        let songs: Vec<String> = SONG_TITLES.iter().map(|s| s.to_string()).collect();
        let song_list = ListBox::builder(&frame)
            .with_choices(songs)
            .with_size(Size::new(240, -1))
            .build();

        // --- Правая панель: заголовок выбранной песни + чарт ---
        let song_title = StaticText::builder(&frame)
            .with_label(SONG_TITLES[0])
            .build();
        let chart_text = TextCtrl::builder(&frame)
            .with_value(CHART)
            .with_style(TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly)
            .build();

        // --- Раскладка: левая колонка фикс., правая растягивается ---
        let left_col = BoxSizer::builder(Orientation::Vertical).build();
        left_col.add(&lib_label, 0, SizerFlag::Expand | SizerFlag::All, 8);
        left_col.add(&song_list, 1, SizerFlag::Expand | SizerFlag::All, 8);

        let right_col = BoxSizer::builder(Orientation::Vertical).build();
        right_col.add(&song_title, 0, SizerFlag::Expand | SizerFlag::All, 8);
        right_col.add(&chart_text, 1, SizerFlag::Expand | SizerFlag::All, 8);

        let root = BoxSizer::builder(Orientation::Horizontal).build();
        root.add_sizer(&left_col, 0, SizerFlag::Expand | SizerFlag::All, 0);
        root.add_sizer(&right_col, 1, SizerFlag::Expand | SizerFlag::All, 0);
        frame.set_sizer(root, true);

        // --- События списка: выбор песни меняет заголовок и статус ---
        // Widgets в wxDragon Copy, поэтому можно спокойно захватывать копии.
        song_list.on_selection_changed(move |event_data| {
            if let Some(name) = event_data.get_string() {
                song_title.set_label(&name);
                frame.set_status_text(&format!("Выбрана песня: {name}"), 0);
            }
        });

        // --- События меню: действия по ID пункта ---
        frame.on_menu_selected(move |event| match event.get_id() {
            ID_OPEN => frame.set_status_text("Открыть… — диалог пока заглушка", 0),
            ID_SAVE => frame.set_status_text("Сохранить… — заглушка, файл не пишется", 0),
            ID_COPY_LOCAL => frame.set_status_text("Копировать — заглушка", 0),
            ID_PASTE_LOCAL => frame.set_status_text("Вставить — заглушка", 0),
            ID_SPEAK => frame.set_status_text("Озвучивание — заглушка (в irealstudio будет TTS)", 0),
            ID_RESET => frame.set_status_text("Готов.", 0),
            ID_ABOUT => {
                frame.set_status_text("wxDragon 0.9.21: wxWidgets 3.3.3, настоящее Win32-меню", 0)
            }
            ID_EXIT => frame.close(true),
            _ => {}
        });

        // --- Подсказки: при проходе стрелками по меню показываем help в статусе ---
        frame.on_menu_highlighted(move |event| {
            let help = match event.get_id() {
                ID_OPEN => "Открыть песню",
                ID_SAVE => "Сохранить изменения",
                ID_COPY_LOCAL => "Скопировать выделенное",
                ID_PASTE_LOCAL => "Вставить из буфера",
                ID_SPEAK => "Прочитать чарт вслух",
                ID_RESET => "Вернуть статус-строку к Готово",
                ID_ABOUT => "Информация о сборке",
                ID_EXIT => "Закрыть программу",
                _ => "",
            };
            if !help.is_empty() {
                frame.set_status_text(help, 0);
            }
        });

        // Выделим первую песню — событие обновит заголовок и статус.
        song_list.set_selection(0, true);

        frame.centre();
        frame.show(true);
    });
}
