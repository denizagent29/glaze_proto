// irealglaze — smoke-test скелет гибрида «нативный менюбар + веб» на Go+Glaze.
//
// Зачем: перед переносом irealstudio на Go надо проверить главный a11y-риск —
// работает ли на Windows классический Win32-менюбар (glaze/menu) ПОВЕРХ
// веб-контента (WebView2 в glaze), доступен ли он по Alt и читает ли его NVDA,
// а также читается ли DOM чарта в странице.
//
// Что здесь показано:
//   - нативное окно glaze, внутри — WebView самой ОС (WebView2 / WKWebView);
//   - нативный менюбар glaze/menu: на Windows цепляется к HWND окна и встаёт
//     строкой сверху над веб-областью (см. README про macOS/Linux);
//   - мост в обе стороны через glaze.Events: меню (Go) -> событие -> JS рисует
//     статус и говорит вслух; кнопка в странице -> JS emit -> Go-обработчик ->
//     ответ обратно в страницу;
//   - прямой вызов страницы из Go через w.Eval;
//   - нативный диалог открытия/сохранения файла;
//   - чарт Rhythm Changes в DOM (текст читается скринридером).
//
// Сборка и запуск на Windows:
//
//	go build -o irealproto.exe .   (или просто go run .)
//
// Требования: Go, WebView2 Runtime (на Windows 10/11 есть по умолчанию).
package main

import (
	"encoding/json"
	"log"
	"runtime"

	"github.com/crgimenes/glaze"
	"github.com/crgimenes/glaze/menu"
)

func init() {
	// UI-поток: окно и его сообщения живут на main-OS-потоке.
	runtime.LockOSThread()
}

// UI держит всё, что нужно колбэкам: окно и мост событий.
type UI struct {
	w  glaze.WebView
	ev *glaze.Events
}

// status отправляет сообщение в страницу (Go -> JS). Безопасно из любой горутины.
func (u *UI) status(msg string) {
	if err := u.ev.Emit("status", msg); err != nil {
		log.Println("status emit:", err)
	}
}

func (u *UI) openAndReport() {
	path, err := u.w.OpenFile(glaze.FileDialogOptions{
		Title:   "Открыть чарт",
		Filters: []glaze.FileFilter{{Name: "iReal Studio", Extensions: []string{"html", "txt"}}},
	})
	if err != nil {
		u.status("Диалог открытия упал: " + err.Error())
		return
	}
	if path == "" {
		u.status("Открытие отменено.")
		return
	}
	u.status("Выбран файл: " + path)
}

func (u *UI) saveAndReport() {
	path, err := u.w.SaveFile(glaze.FileDialogOptions{
		Title:    "Экспорт iReal Pro HTML",
		Filename: "Rhythm_Changes.html",
		Filters:  []glaze.FileFilter{{Name: "HTML", Extensions: []string{"html"}}},
	})
	if err != nil {
		u.status("Диалог сохранения упал: " + err.Error())
		return
	}
	if path == "" {
		u.status("Сохранение отменено.")
		return
	}
	u.status("Сохранено: " + path)
}

// editItems строит меню «Правка». На macOS нативные селекторы дают Cmd+C/V/X
// дорогу к сфокусированному WebView (иначе шорткаты не доходят). На Windows
// редакторские команды обрабатывает сам WebView2: пункты меню просто посылают
// туда команду через JS, чтобы меню не пустовало и было видно в NVDA.
func editItems(w glaze.WebView) []menu.Item {
	if runtime.GOOS == "darwin" {
		return []menu.Item{
			{Title: "Отменить", Selector: "undo:"},
			{Title: "Повторить", Selector: "redo:"},
			{Separator: true},
			{Title: "Вырезать", Selector: "cut:"},
			{Title: "Копировать", Selector: "copy:"},
			{Title: "Вставить", Selector: "paste:"},
			{Title: "Выделить всё", Selector: "selectAll:"},
		}
	}
	cmd := func(js string) func() { return func() { w.Eval(js) } }
	return []menu.Item{
		{Title: "Отменить", Shortcut: "ctrl+z", OnClick: cmd("document.execCommand('undo')")},
		{Title: "Повторить", Shortcut: "ctrl+y", OnClick: cmd("document.execCommand('redo')")},
		{Separator: true},
		{Title: "Вырезать", Shortcut: "ctrl+x", OnClick: cmd("document.execCommand('cut')")},
		{Title: "Копировать", Shortcut: "ctrl+c", OnClick: cmd("document.execCommand('copy')")},
		{Title: "Вставить", Shortcut: "ctrl+v", OnClick: cmd("document.execCommand('paste')")},
		{Title: "Выделить всё", Shortcut: "ctrl+a", OnClick: cmd("document.execCommand('selectAll')")},
	}
}

func main() {
	runtime.LockOSThread()

	w, err := glaze.New(false) // false — без devtools
	if err != nil {
		log.Fatal("glaze.New:", err)
	}
	defer w.Destroy()

	w.SetTitle("irealstudio — glaze-прототип: менюбар + веб")
	w.SetSize(980, 680, glaze.HintNone)

	// Мост Go<->JS: ставит window.glaze.events в странице. Вызывать до Run.
	ev, err := glaze.NewEvents(w)
	if err != nil {
		log.Fatal("NewEvents:", err)
	}
	u := &UI{w: w, ev: ev}

	// ---- Страница -> Go. Обработчики JS-событий бегут на binding-горутине:
	// всё, что трогает окно/диалоги, заворачиваем в w.Dispatch (UI-поток).
	ev.On("ui:ping", func(args ...json.RawMessage) {
		who := "страницы"
		if len(args) > 0 && len(args[0]) > 0 {
			var s string
			if json.Unmarshal(args[0], &s) == nil {
				who = s
			}
		}
		u.status("Пинг от " + who + " дошёл до Go.")
	})
	ev.On("ui:open", func(args ...json.RawMessage) {
		w.Dispatch(u.openAndReport)
	})

	// ---- Менюбар. Ставим ДО Run, на UI-потоке, Dispatch=nil (иначе клин).
	_, err = menu.Set([]menu.Item{
		{Title: "Файл", Submenu: []menu.Item{
			{Title: "Новый чарт", Shortcut: "ctrl+n", OnClick: func() {
				u.status("Файл → Новый (заглушка).")
			}},
			{Title: "Открыть…", Shortcut: "ctrl+o", OnClick: u.openAndReport},
			{Separator: true},
			{Title: "Экспорт iReal Pro HTML…", Shortcut: "ctrl+e", OnClick: u.saveAndReport},
			{Separator: true},
			{Title: "Выход", Shortcut: "ctrl+q", OnClick: func() { w.Terminate() }},
		}},
		{Title: "Правка", Submenu: editItems(w)},
		{Title: "Вид", Submenu: []menu.Item{
			{Title: "Фокус в чарт", Shortcut: "ctrl+l", OnClick: func() {
				w.Focus()
				u.status("Фокус передан в веб-контент.")
			}},
			{Title: "Озвучить чарт", Shortcut: "ctrl+t", OnClick: func() {
				_ = ev.Emit("tts", chartForSpeech)
			}},
			{Separator: true},
			// Бисект NVDA-тормозов: тормозит ли Alt-меню на пустой странице?
			{Title: "Тест: пустая страница (проверка Alt-меню)", OnClick: func() {
				w.Navigate("about:blank")
				u.status("Пустая страница. Жми Alt и засеки, виснет ли меню.")
			}},
			{Title: "Тест: показать чарт", OnClick: func() {
				w.SetHtml(pageHTML)
				u.status("Чарт снова загружен.")
			}},
		}},
		{Title: "Демо", Submenu: []menu.Item{
			{Title: "w.Eval в страницу", OnClick: func() {
				w.Eval("demoEval()")
			}},
			{Title: "Сброс статуса", OnClick: func() {
				w.Eval("setStatus('')")
			}},
			{Disabled: true, Title: "Пункт-заглушка (disabled)"},
		}},
		{Title: "Справка", Submenu: []menu.Item{
			{Title: "О прототипе", OnClick: func() {
				u.status("Glaze-скелет: нативный менюбар + WebView. Собран для проверки Alt/NVDA.")
			}},
		}},
	}, menu.Options{Window: w.Window()})
	if err != nil {
		// На Linux меню не поддержано (ErrUnsupported) — там не fatal: окно открывается.
		log.Println("menu.Set:", err)
	}

	w.SetHtml(pageHTML)
	w.Run()
}

// chartForSpeech — короткий озвученный пересказ чарта (числа словами).
const chartForSpeech = "Ритм Чейнджес, ми-бемоль мажор, ап-темпо-свинг. " +
	"Форма: А, А, Б, А, тридцать два такта. " +
	"Куплет А: ми-бемоль-шесть два такта, ми-бемоль-септ два такта, " +
	"ля-бемоль-септ два такта, ми-бемоль-шесть, и в конце оборот фа-минор-септ, си-бемоль-септ. " +
	"Бридж Б: соль-септ, соль-септ, до-септ, до-септ, фа-септ, фа-септ, си-бемоль-септ два такта."

// pageHTML — тело приложения. Чарт — настоящий DOM (текст), не canvas, чтобы
// NVDA читал аккорды; статусная строка aria-live объявляет события из меню.
const pageHTML = `<!doctype html>
<html lang="ru">
<head>
<meta charset="utf-8">
<title>irealstudio prototype</title>
<style>
  body { font-family: system-ui, -apple-system, sans-serif; background:#0f172a;
         color:#e2e8f0; margin:0; padding:1.2rem 1.5rem 2rem; }
  h1 { font-size:19px; margin:0 0 .3rem; }
  .sub { color:#94a3b8; font-size:13px; margin-bottom:1rem; }
  section.chart { background:#1e293b; border:1px solid #334155; border-radius:10px;
         padding:.8rem 1rem; }
  .sec { margin:.6rem 0 .2rem; font-weight:600; color:#7dd3fc; }
  .bar { display:block; font-size:16px; letter-spacing:.02em; margin:.15rem 0; }
  .m { display:inline-block; min-width:4.2em; }
  .hint { color:#64748b; font-size:12px; }
  #status { display:block; margin-top:1rem; padding:.6rem .8rem; background:#111827;
         border:1px solid #374151; border-radius:8px; min-height:1.2em; color:#fbbf24; }
  button { margin-right:.5rem; margin-top:.7rem; padding:.4em .8em; font-size:14px;
         border-radius:6px; border:1px solid #475569; background:#1e293b; color:#e2e8f0; }
  button:hover { background:#334155; }
</style>
</head>
<body>
  <h1>Rhythm Changes</h1>
  <div class="sub">ми-бемоль мажор · Up Tempo Swing · 200 bpm · А-А-Б-А, 32 такта
    — демо для проверки меню и скринридера</div>

  <section class="chart" aria-label="Чарт Ритм Чейнджес">
    <div class="sec">A</div>
    <span class="bar"><span class="m">Eb6</span><span class="m">Eb6</span><span class="m">Eb7</span><span class="m">Eb7</span><span class="m">Ab7</span><span class="m">Ab7</span><span class="m">Eb6</span><span class="m">Fm7 Bb7</span></span>
    <div class="sec">A</div>
    <span class="bar"><span class="m">Eb6</span><span class="m">Eb6</span><span class="m">Eb7</span><span class="m">Eb7</span><span class="m">Ab7</span><span class="m">Ab7</span><span class="m">Eb6</span><span class="m">Fm7 Bb7</span></span>
    <div class="sec">B</div>
    <span class="bar"><span class="m">G7</span><span class="m">G7</span><span class="m">C7</span><span class="m">C7</span><span class="m">F7</span><span class="m">F7</span><span class="m">Bb7</span><span class="m">Bb7</span></span>
    <div class="sec">A</div>
    <span class="bar"><span class="m">Eb6</span><span class="m">Eb6</span><span class="m">Eb7</span><span class="m">Eb7</span><span class="m">Ab7</span><span class="m">Ab7</span><span class="m">Fm7 Bb7</span><span class="m">Eb6</span></span>
  </section>

  <button type="button" id="ping">Позвать Go (пинг)</button>
  <button type="button" id="opn">Открыть файл (диалог Go)</button>

  <div class="hint" style="margin-top:.9rem">Меню сверху — нативное (Win32), его читает NVDA,
    навигация по Alt. Сообщения меню приходят сюда и озвучиваются.</div>
  <output id="status" aria-live="polite" role="status">Готово. Жми Alt, чтобы открыть меню.</output>

<script>
  // Прямой вызов из Go через w.Eval.
  window.demoEval = function () {
    setStatus('w.Eval: Go напрямую вызвал JS в ' + new Date().toLocaleTimeString('ru-RU'));
  };
  function setStatus(t) { document.getElementById('status').textContent = t; }
  function say(t) {
    if (!('speechSynthesis' in window)) return;
    window.speechSynthesis.cancel();
    var u = new SpeechSynthesisUtterance(t);
    u.lang = 'ru-RU';
    window.speechSynthesis.speak(u);
  }

  function boot() {
    if (!(window.glaze && window.glaze.events)) { setTimeout(boot, 50); return; }
    var e = window.glaze.events;
    // Go -> JS: статус в строку (aria-live сама объявит), tts — говорить вслух.
    e.on('status', function (msg) { setStatus(msg); });
    e.on('tts', function (text) { say(text); });

    document.getElementById('ping').addEventListener('click', function () {
      e.emit('ui:ping', 'кнопки «пинг»');
    });
    document.getElementById('opn').addEventListener('click', function () {
      e.emit('ui:open');
    });
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', boot);
  } else { boot(); }
</script>
</body>
</html>`
