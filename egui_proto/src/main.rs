// irealstudio — a11y smoke-прототип на Rust + egui.
//
// Цель: проверить, читает ли NVDA egui-интерфейс (меню, кнопки, список,
// текст) через AccessKit. egui сам по себе NVDA не отдаёт ничего, но с
// фичей `accesskit` (включена в Cargo.toml) на Windows поднимается
// accesskit_windows, который выставляет UIA-провайдер на окне — NVDA читает
// его как обычное настольное приложение. Дерево не переключается, как в
// случае WebView2, поэтому фриза, как в Go/Glaze-прототипе, быть не должно.
//
// Сборка на Windows:  cargo run --release
// Первая сборка долгая (тянет eframe/egui/accesskit) — это нормально.
// Консоль не прячем: если eframe не поднимется (winit/accesskit), ошибка
// будет видна в stdout, а не проглочена.

use eframe::egui;

/// Псевдо-библиотека стандартов. Выбор песни пока только меняет заголовок.
const SONG_TITLES: &[&str] = &[
    "Rhythm Changes",
    "All The Things You Are",
    "Autumn Leaves",
    "Blue Bossa",
    "Giant Steps",
    "Take Five",
];

/// Образец чарта (как будет выглядеть текстовая расшифровка). Символы аккордов
/// здесь — нотация, не текст для озвучки, поэтому буквы/цифры не «прописью».
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

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("irealstudio — прототип egui")
            .with_inner_size(egui::vec2(920.0, 640.0)),
        ..Default::default()
    };
    eframe::run_native(
        "irealstudio_egui_proto",
        options,
        Box::new(|cc| Box::new(ProtoApp::new(cc))),
    )
}

struct ProtoApp {
    selected: usize,
    query: String,
    status: String,
}

impl ProtoApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_system_fonts(&cc.egui_ctx);
        Self {
            selected: 0,
            query: String::new(),
            status: "Прототип egui: меню, плейлист, чарт. Жми Tab — NVDA должна читать элементы."
                .to_owned(),
        }
    }

    fn set_status(&mut self, text: impl Into<String>) {
        self.status = text.into();
    }
}

impl eframe::App for ProtoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Менюбар (нативный для окна, но рисованный egui) ---
        egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Файл", |ui| {
                    if ui.button("Открыть…").clicked() {
                        self.set_status("Открыть… — диалог пока заглушка");
                        ui.close_menu();
                    }
                    if ui.button("Сохранить…").clicked() {
                        self.set_status("Сохранить… — заглушка, файл не пишется");
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Выход").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Правка", |ui| {
                    if ui.button("Копировать").clicked() {
                        self.set_status("Копировать — заглушка");
                        ui.close_menu();
                    }
                    if ui.button("Вставить").clicked() {
                        self.set_status("Вставить — заглушка");
                        ui.close_menu();
                    }
                });

                ui.menu_button("Вид", |ui| {
                    if ui.button("Озвучить чарт").clicked() {
                        self.set_status("Озвучивание — заглушка (в irealstudio будет TTS)");
                        ui.close_menu();
                    }
                    if ui.button("Сбросить статус").clicked() {
                        self.set_status("Готов.");
                        ui.close_menu();
                    }
                });

                ui.menu_button("Справка", |ui| {
                    if ui.button("О прототипе").clicked() {
                        self.set_status("egui 0.27.2: accesskit -> UIA, читает NVDA");
                        ui.close_menu();
                    }
                });
            });
        });

        // --- Статусная строка внизу ---
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.add(
                egui::Label::new(egui::RichText::new(&self.status).strong()).wrap(true),
            );
        });

        // --- Слева: библиотека песен ---
        egui::SidePanel::left("playlist")
            .resizable(true)
            .default_width(230.0)
            .show(ctx, |ui| {
                ui.heading("Библиотека");
                ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Поиск… (поле для проверки ввода)"),
                );
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, name) in SONG_TITLES.iter().enumerate() {
                        ui.selectable_value(&mut self.selected, i, *name);
                    }
                });
            });

        // --- Центр: чарт выбранной песни ---
        egui::CentralPanel::default().show(ctx, |ui| {
            self.selected = self.selected.min(SONG_TITLES.len() - 1);
            let title = SONG_TITLES[self.selected];
            ui.heading(title);
            ui.label("Меню — стрелки + Enter, выход из меню — Esc. Tab обходит кнопки и список.");
            ui.horizontal(|ui| {
                if ui.button("Озвучить чарт").clicked() {
                    self.set_status("Озвучивание — заглушка");
                }
                if ui.button("Открыть файл…").clicked() {
                    self.set_status("Открыть файл — заглушка");
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(CHART).monospace().size(16.0),
                    )
                    .wrap(true),
                );
            });
        });
    }
}

/// Подтягивает системный шрифт с кириллицей (Segoe UI на Windows) как fallback.
/// Встроенные шрифты egui кириллицу не гарантируют, а весь UI на русском.
/// Для NVDA это не критично (текст читается из дерева, не с экрана), но чтобы
/// окно не выглядело «квадратами» у зрячего наблюдателя — грузим Segoe UI.
fn install_system_fonts(ctx: &egui::Context) {
    #[cfg(target_os = "windows")]
    {
        let mut fonts = egui::FontDefinitions::default();
        let mut added = false;
        for path in [
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                fonts.font_data
                    .insert("system_ui".to_owned(), egui::FontData::from_owned(bytes));
                for family in [
                    egui::FontFamily::Proportional,
                    egui::FontFamily::Monospace,
                ] {
                    fonts
                        .families
                        .entry(family)
                        .or_default()
                        .push("system_ui".to_owned());
                }
                added = true;
                break;
            }
        }
        if added {
            ctx.set_fonts(fonts);
        }
    }
    let _ = ctx;
}
