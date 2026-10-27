app-title = Revel
app-tagline = Визуальный конструктор сайтов
app-version = Версия { $version }
app-stage = Этап { $stage }: модульный каркас

menu-file = Файл
menu-site = Сайт
menu-publish = Публикация
menu-help = Справка

language = Язык

deploy-warn-no-301 = Этот хостинг не умеет отправлять настоящие редиректы 301: перенаправления ({ $count }) будут выполняться на стороне браузера, а поисковые системы обрабатывают их хуже.
deploy-warn-no-php = Этот хостинг не выполняет PHP: файлы PHP (например, контактная форма) работать не будут, их количество: { $count }.
deploy-warn-no-headers = Этот хостинг не позволяет задавать свои HTTP-заголовки (индексация, кэш и безопасность).
deploy-warn-no-htaccess = Этот хостинг игнорирует файлы .htaccess: их правила (HTTPS, перенаправления, доступ) применяться не будут.
deploy-warn-site-too-big = Размер сайта ({ $size } MB) превышает лимит этого хостинга ({ $max } MB).
deploy-warn-file-too-big = Размер файла ({ $size } MB) превышает лимит этого хостинга на один файл ({ $max } MB).
deploy-warn-too-many-files = На сайте файлов: { $count }, а этот хостинг принимает не более { $max }.
deploy-warn-public-repo = На бесплатном тарифе этот хостинг требует, чтобы репозиторий с исходниками был публичным.

home-title = Сайты
home-empty = Сайтов пока нет
home-drop = Перетащите папку, чтобы добавить сайт
gh-sync = Синхронизировать
gh-sync-help = Загружает на этот компьютер версии ваших сайтов с GitHub и отправляет те, что сохранены здесь; показывает сайты аккаунта, которых здесь ещё нет
gh-sync-updated = Обновлено: { $n }
gh-sync-sent = Отправлено: { $n }
gh-sync-waiting = Проверить при открытии сайта: { $n }
gh-sync-failed = Ошибок: { $n }
gh-sync-none = Всё актуально
gh-site-download = Скачать на этот компьютер
gh-site-download-open = Скачать и открыть
gh-site-github = Открыть на GitHub
action-open = Открыть сайт
action-import = Импортировать
action-new = Новый сайт
action-forget = Убрать из списка
action-cancel = Отмена
action-save = Сохранить
action-delete = Удалить
new-site-name = Название сайта
import-overwrite = В этой папке уже есть сайт Revel. Повторный импорт заменит его структуру (страницы, макеты, коллекции); содержимое и изображения не изменятся.
import-done = Импорт завершён
import-warnings = Что нужно проверить
site-pages = Страницы
site-layouts = Макеты
site-components = Компоненты
site-collections = Коллекции
site-entries = { $count ->
        [one] { $count } запись
        [few] { $count } записи
        [many] { $count } записей
       *[other] { $count } записи
    }
build-env = Окружение
build-images = Адаптивные изображения (AVIF, WebP)
build-phase-pages = Страницы
build-phase-copies = Файлы
build-phase-images = Изображения { $done }/{ $total }
build-done = Создано страниц: { $generated }, скопировано файлов: { $copied }, изображений: { $produced } новых и { $cached } из кэша, за { $seconds } с.
build-warnings = Предупреждения
preview-component = Компонент · { $name }
preview-open = Предпросмотр
publish-empty = Для этого сайта хостинг ещё не добавлен. Добавьте SFTP-сервер (OVH, o2switch…) или GitHub Pages, чтобы публиковать в один клик.
publish-add = Добавить хостинг
publish-name = Название
publish-env = Созданное окружение
publish-host = SFTP-сервер
publish-port = Порт
publish-user = Логин
publish-remote-dir = Папка на сервере
publish-password = Пароль
publish-password-saved = Уже сохранён в системной связке ключей. Оставьте поле пустым, чтобы не менять.
publish-check = Проверить
publish-run = Опубликовать
publish-planned = { $upload ->
        [one] { $upload } файл к отправке
        [few] { $upload } файла к отправке
        [many] { $upload } файлов к отправке
       *[other] { $upload } файла к отправке
    } ({ $size } MB), на удаление: { $delete }, всего файлов на сайте: { $total }.
publish-progress = Отправка: { $done } / { $total } { $total ->
        [one] файл
        [few] файла
        [many] файлов
       *[other] файла
    }
publish-done = { $uploaded ->
        [0] { $deleted ->
            [0] Публиковать нечего: сайт в сети уже актуален.
            [one] Опубликовано: удалён { $deleted } файл.
            [few] Опубликовано: удалено { $deleted } файла.
            [many] Опубликовано: удалено { $deleted } файлов.
           *[other] Опубликовано: удалено { $deleted } файла.
        }
        [one] Опубликовано: отправлен { $uploaded } файл, удалено: { $deleted }.
        [few] Опубликовано: отправлено { $uploaded } файла, удалено: { $deleted }.
        [many] Опубликовано: отправлено { $uploaded } файлов, удалено: { $deleted }.
       *[other] Опубликовано: отправлено { $uploaded } файла, удалено: { $deleted }.
    }
publish-dry-done = { $upload ->
        [0] { $delete ->
            [0] Проверка завершена: сайт в сети уже актуален.
            [one] Проверка завершена, на сервере ничего не изменено. При следующей публикации будет удалён { $delete } файл.
            [few] Проверка завершена, на сервере ничего не изменено. При следующей публикации будет удалено { $delete } файла.
            [many] Проверка завершена, на сервере ничего не изменено. При следующей публикации будет удалено { $delete } файлов.
           *[other] Проверка завершена, на сервере ничего не изменено. При следующей публикации будет удалено { $delete } файла.
        }
        [one] Проверка завершена, на сервере ничего не изменено. При следующей публикации будет отправлен { $upload } файл ({ $size } MB), удалено: { $delete }.
        [few] Проверка завершена, на сервере ничего не изменено. При следующей публикации будет отправлено { $upload } файла ({ $size } MB), удалено: { $delete }.
        [many] Проверка завершена, на сервере ничего не изменено. При следующей публикации будет отправлено { $upload } файлов ({ $size } MB), удалено: { $delete }.
       *[other] Проверка завершена, на сервере ничего не изменено. При следующей публикации будет отправлено { $upload } файла ({ $size } MB), удалено: { $delete }.
    }
publish-key-recorded = Первое подключение: ключ сервера сохранён и будет требоваться при следующих подключениях. Сравните его отпечаток с тем, который показывает ваш хостинг:
publish-failed = Не удалось опубликовать. { $message }
publish-warnings = Перед публикацией
error = Ошибка: { $message }
nav-recent = Недавние
nav-all-sites = Все сайты
nav-publish = Публикация
# Visual editor (step 2)
editor-loading = Открываем сайт в редакторе…
editor-canvas = Холст страницы
editor-page = Отображаемая страница
editor-breakpoints = Брейкпоинты
editor-bp-desktop = Компьютер (основной)
editor-bp-max = До { $width } px
editor-bp-min = От { $width } px
editor-zoom = Масштаб
editor-undo = Отменить (Ctrl+Z)
editor-redo = Повторить (Ctrl+Shift+Z)
editor-no-selection = Ничего не выбрано
editor-bp-toggle = { $name }: показать или скрыть на холсте
editor-frame-height = Высота страницы
editor-frame-height-help = Текущая высота: { $height } px. Перетащите, чтобы увеличить или уменьшить; двойной щелчок: высота по умолчанию
editor-side = Все экраны
editor-dark = Предпросмотр тёмной темы
editor-dark-help = Показать сайт с цветами тёмной темы, как на устройстве с тёмным оформлением
editor-free = Свободный холст
editor-free-help = Свободный холст: перемещайте вид, меняйте масштаб, задавайте ширину и высоту кадра. Выключено: страница подгоняется под область, колесо мыши её прокручивает
editor-side-help = Показывает все экраны рядом; щёлкните внутри экрана, чтобы выбрать его
palette = Команды
palette-placeholder = Найти действие, элемент, страницу…
palette-none = Ничего не найдено
palette-actions = Действия
palette-view = Вид
palette-add = Добавить
palette-goto = Перейти
palette-pages = Страницы
palette-screens = Экраны
palette-layers = Слои страницы
keys-next = Следующий элемент
keys-prev = Предыдущий элемент
keys-up = Переместить элемент вверх
keys-down = Переместить элемент вниз
keys-zoom-in = Увеличить
keys-zoom-out = Уменьшить
keys-shift = Shift
keys-delete = Delete
keys-escape = Esc
keys-enter = Enter
keys-space = Пробел
prefs-editor = Редактор
prefs-editor-help = Связи между вариантами компонентов и группы, предлагаемые в быстром поиске (Ctrl+K)
prefs-variants = Варианты компонентов
prefs-links = Связи
prefs-links-help = Форма связей от основного варианта к вариантам, вызываемым жестом
prefs-links-curve = Кривые
prefs-links-square = Прямоугольные
prefs-links-straight = Прямые
prefs-link-end = Конец
prefs-link-end-arrow = Стрелка
prefs-link-end-dot = Точка
prefs-palette = Быстрый поиск
prefs-palette-help = Группы результатов, показываемые в быстром поиске (Ctrl+K)
prefs-colors = Цвета интерфейса
prefs-colors-help = Цветовой код редактора, который не заменяется акцентным цветом: компоненты, значение, заданное здесь, унаследованное значение, машинный перевод для проверки
prefs-color-comp = Компоненты
prefs-color-set = Задано здесь
prefs-color-inherit = Унаследовано
prefs-color-auto = Машинный перевод
prefs-color-reset = Цвет по умолчанию
prefs-shortcuts = Сочетания клавиш
prefs-shortcuts-help = Клавиши каждого действия редактора: нажмите «Изменить», затем нужное сочетание (Esc отменяет)
prefs-key-press = Нажмите клавиши…
prefs-key-none = Нет
prefs-key-change = Изменить
prefs-key-clear = Удалить сочетание
prefs-key-reset = Клавиши по умолчанию
prefs-key-reset-all = Сбросить всё
prefs-key-conflict = Те же клавиши у: { $names }
key-palette = Ctrl+K · Ctrl+Space
editor-multi = Выбрано элементов: { $count }
editor-wrap-siblings = Чтобы обернуть выбранные элементы вместе, у них должен быть один родитель.
editor-multi-add = Добавить к выделению
key-multi = Shift+щелчок
editor-resize = Размер
key-resize = Перетащите маркер
editor-owner-page = Страница { $name }
editor-owner-layout = Макет { $name }
editor-owner-component = Компонент { $name }
editor-saving = Сохранение…
editor-saved = Сохранено в { $time }
editor-saved-auto = Сохраняется автоматически
editor-shortcuts = Сочетания клавиш
editor-undo-short = Отменить
editor-redo-short = Повторить
layers-duplicate-short = Дублировать элемент
layers-delete-short = Удалить элемент
editor-deselect = Снять выделение
editor-edit-text = Изменить текст страницы
editor-edit-validate = Подтвердить текст (Esc: отмена)
key-undo = Ctrl+Z
key-redo = Ctrl+Shift+Z · Ctrl+Y
key-duplicate = Ctrl+D
key-delete = Delete
key-escape = Esc
key-dblclick = Двойной щелчок
key-enter = Enter
layers-title = Слои
layers-up = Переместить выше
layers-down = Переместить ниже
layers-wrap = Обернуть в контейнер
layers-duplicate = Дублировать
layers-delete = Удалить
layers-if = Условие · { $cond }
layers-if-help = Показывается при выполнении условия ({ $cond }), иначе показывается содержимое блока «Иначе»
layers-else = Иначе
layers-each = Список · { $list }
layers-each-help = Повторяется для каждого элемента списка ({ $item } из { $list })
layers-condition = Показывается при условии: { $cond }
layers-bound = Текст из данных сайта или записи CMS
layers-edit-text = Двойной щелчок: изменить текст
layers-empty-list = если список пуст
layers-image = Изображение
layers-html = HTML
layers-slot = Содержимое страницы
layers-markdown = Содержимое (Markdown)
layers-animated = Анимирован (панель «Взаимодействия»)
layers-rename = Переименовать
layers-hide = Скрыть
layers-show = Показать
key-hide = Ctrl+Shift+H
layers-hidden = Скрыт: на опубликованном сайте его не будет
style-hide = Скрыть слой
style-show = Показать слой
el-heading = Заголовок { $level }
el-section = Секция
el-div = Блок
el-p = Абзац
el-a = Ссылка
el-button = Кнопка
el-img = Изображение
el-svg = Значок
el-video = Видео
el-audio = Аудио
el-iframe = Встраиваемый контент
el-ul = Список
el-ol = Нумерованный список
el-li = Элемент списка
el-nav = Навигация
el-header = Шапка
el-footer = Подвал
el-main = Основное содержимое
el-article = Статья
el-aside = Боковая панель
el-form = Форма
el-input = Поле
el-textarea = Многострочное поле
el-select = Выпадающий список
el-label = Подпись поля
el-span = Фрагмент текста
el-strong = Жирный
el-em = Курсив
el-blockquote = Цитата
el-figure = Иллюстрация
el-figcaption = Подпись к иллюстрации
el-table = Таблица
el-hr = Разделитель
el-body = Тело страницы
el-code = Код
el-small = Мелкий текст
el-time = Дата
el-address = Адрес
el-details = Раскрывающийся блок
el-summary = Заголовок раскрывающегося блока
el-dialog = Диалоговое окно
el-other = Элемент
layers-badge-component = компонент
insert-title = Добавить
insert-no-place = Выберите на странице элемент, рядом с которым нужно разместить новый.
insert-section = Секция
insert-container = Контейнер
insert-heading = Заголовок
insert-paragraph = Абзац
insert-link = Ссылка
insert-button = Кнопка
insert-image = Изображение
insert-list = Список
insert-text-heading = Новый заголовок
insert-text-paragraph = Новый абзац.
insert-text-link = Ссылка
insert-text-button = Кнопка
insert-text-image = Изображение
insert-text-list = Элемент списка
inspector-title = Инспектор
inspector-settings = Настройки
inspector-advanced = Дополнительные настройки
inspector-content-classes = Классы по содержимому
inspector-content-classes-help = Классы, добавляемые по полям содержимого (заполненное поле, значение поля), у каждого свой стиль в «Стилях»
inspector-none = Выберите элемент на странице или в слоях.
inspector-tag = Тег
inspector-id = Идентификатор (id)
inspector-classes = Классы
inspector-class-add = Добавить класс
inspector-class-remove = Удалить класс
inspector-link = Ссылка
inspector-href = Адрес
inspector-heading-level = Уровень
link-section = Секция
link-section-help = Элемент страницы с идентификатором (задаётся в дополнительных настройках)
link-section-choose = Выберите секцию…
link-section-none = Нет секций (задайте элементу идентификатор)
link-file = Файл
link-kind = Тип ссылки
link-page = Страница
link-url = URL
link-email = Эл. почта
link-phone = Телефон
link-page-choose = Выберите страницу…
inspector-new-tab = Открывать в новой вкладке
inspector-current-section = Активна на вложенных страницах
inspector-current-section-help = Ссылка также получает состояние «Текущая страница» на страницах, адрес которых начинается с её адреса (/projects/ для /projects/my-project/). На своей странице она получает его всегда.
inspector-attributes = Другие атрибуты
inspector-attr-add = Добавить: имя=значение
inspector-attr-remove = Удалить атрибут
inspector-attr-boolean = (без значения)
attr-placeholder = Текст-подсказка
attr-aria-label = Название для программ экранного доступа
attr-aria-hidden = Скрыто от программ экранного доступа
attr-aria-describedby = Описывается элементом
attr-role = Роль (доступность)
attr-title = Всплывающая подсказка
attr-data-src = Источник (загружается позже)
attr-src = Источник
attr-srcset = Размеры изображения
attr-loading = Загрузка
attr-rel = Тип связи ссылки
attr-download = Скачивание
attr-tabindex = Порядок обхода с клавиатуры
attr-type = Тип
attr-name = Имя
attr-value = Значение
attr-width = Ширина
attr-height = Высота
attr-lang = Язык
attr-datetime = Дата (машинный формат)
attr-for = Связанное поле
attr-action = Адрес отправки
attr-method = Способ отправки
attr-allow = Разрешения
attr-frameborder = Граница фрейма
attr-viewbox = Область рисунка
attr-decoding = Декодирование
attr-fetchpriority = Приоритет загрузки
inspector-attr-when = Записывается только если: { $cond }
inspector-src = Изображение (адрес)
inspector-src-expr = Изображение (выражение)
inspector-alt = Альтернативный текст
inspector-alt-expr = Альтернативный текст (выражение)
inspector-condition = Условие
inspector-each-list = Список
inspector-each-item = Имя каждого элемента
inspector-component = Компонент { $name }: его элементы настраиваются в слоях.
editor-bp-manage = Управление брейкпоинтами
editor-bp-remove = Удалить этот брейкпоинт
editor-bp-add = Добавить
editor-bp-kind-max = До
editor-bp-kind-min = От
editor-bp-in-use = У этого брейкпоинта есть стили: сначала удалите их на панели стилей.
styles-title = Стили
styles-selector = Класс
styles-state = Состояние
styles-state-none = По умолчанию
styles-state-hover = Наведение
styles-state-focus = Фокус клавиатуры
styles-state-current = Текущая страница
styles-state-is-active = Активен
styles-state-in-active = Родитель активен
styles-state-open = Открыт
styles-state-in-open = Аккордеон открыт
styles-state-off = Выключен
styles-state-in-off = Переключатель выключен
styles-state-active = Нажатие
styles-state-none-help = Стили элемента в обычном состоянии.
styles-state-hover-help = Указатель над элементом.
styles-state-active-help = Пока элемент нажат.
styles-state-focus-help = Элемент выбран с клавиатуры (клавиша Tab).
styles-state-current-help = Ссылка на открытую страницу (меню).
styles-state-is-active-help = Вкладка, фильтр или включённый переключатель, элемент под курсором в медиа при наведении или случайно выбранный элемент.
styles-state-in-active-help = Элемент внутри активного элемента (текст активной вкладки).
styles-state-open-help = Открытый аккордеон: во время редактирования холст открывает его сам.
styles-state-in-open-help = Элемент внутри открытого аккордеона.
styles-state-off-help = Переключатель звука или музыки в выключенном положении.
styles-state-in-off-help = Элемент внутри выключенного переключателя.
styles-state-ready-help = Медиа при наведении запущено (видео) или загружено (изображение).
styles-state-in-paused-help = Элемент внутри элемента, остановленного действием «Воспроизвести или приостановить».
styles-class-new = Добавить класс…
styles-class-suggest = Предлагаемые классы
styles-class-create = Создать «{ $name }»
class-why-sibling = Используется соседним элементом того же типа
class-why-family = Назван по классу родителя
class-why-neighbour = Используется соседним элементом
class-why-tag = Используется в другом месте элементами этого типа
class-why-other = Другой класс сайта
styles-tag-help = Задаёт стили для каждого тега этого типа на сайте, без класса. Если не выбрать этот вариант, первое заданное значение присвоит элементу класс.
styles-tag-body = Тело страницы (все страницы)
styles-tag-p = Все абзацы
styles-tag-h1 = Все заголовки H1
styles-tag-h2 = Все заголовки H2
styles-tag-h3 = Все заголовки H3
styles-tag-h4 = Все заголовки H4
styles-tag-h5 = Все заголовки H5
styles-tag-h6 = Все заголовки H6
styles-tag-a = Все ссылки
styles-tag-img = Все изображения
styles-tag-ul = Все маркированные списки
styles-tag-ol = Все нумерованные списки
styles-tag-li = Все элементы списка
styles-tag-blockquote = Все блоки цитат
styles-tag-strong = Весь жирный текст
styles-tag-em = Весь курсивный текст
styles-tag-figure = Все иллюстрации
styles-tag-section = Все секции
styles-tag-button = Все кнопки
styles-tag-video = Все видео
styles-no-class = У этого элемента нет класса: добавьте его, чтобы задать стили.
styles-new-rule = Здесь пока ничего не задано: первое значение создаст стиль, который применяется поверх более широких экранов.
styles-elements-only = Стили применяются к элементам HTML.
styles-reset = Сбросить
styles-important = Приоритет (!important)
styles-declarations = Код CSS (для опытных)
styles-decl-add = Добавить: свойство: значение
styles-layout = Раскладка
styles-spacing = Отступы
styles-size = Размер
styles-position = Положение
styles-typography = Текст
styles-backgrounds = Фон
styles-borders = Граница
styles-effects = Эффекты
styles-display = Отображение
styles-direction = Направление
styles-align = Выравнивание
styles-gap = Промежуток
styles-columns = Колонки
styles-margin = Внешний отступ
styles-padding = Внутренний отступ
styles-width = Ширина
styles-height = Высота
styles-min-width = Мин. ширина
styles-max-width = Макс. ширина
styles-min-height = Мин. высота
styles-max-height = Макс. высота
styles-overflow = Переполнение
styles-position-type = Тип
styles-z-index = Порядок слоёв
text-field = Текст
text-field-help = Текст элемента; Enter подтверждает, Shift+Enter переносит строку. Его также можно изменить двойным щелчком на странице.
text-field-data = Текст из данных сайта: изменяется везде, где показан
text-field-entry = Текст записи CMS: изменяется в этой записи
text-field-prop = Текстовое свойство компонента: изменяется для этого экземпляра
styles-font-family = Шрифт
styles-font-size = Размер
styles-font-weight = Насыщенность
styles-line-height = Высота строки
styles-letter-spacing = Межбуквенный интервал
styles-color = Цвет
styles-text-align = Выравнивание
styles-text-transform = Регистр
styles-background-color = Цвет
styles-background-image = Изображение и градиент
styles-background-size = Размер изображения
styles-background-position = Положение
styles-border-width = Толщина
styles-border-style = Стиль линии
styles-border-color = Цвет
styles-border-radius = Радиус скругления
styles-opacity = Непрозрачность
styles-box-shadow = Тени
nav-settings = Настройки
home-col-name = Название
home-col-folder = Папка
home-col-opened = Последнее открытие
settings-lang = Язык
settings-theme = Тема
theme-system = Системная
theme-dark = Тёмная
theme-light = Светлая
settings-accent = Акцентный цвет
settings-accent-help = Цвет интерфейса; какие элементы его используют, определяется в разделе «Охват акцентного цвета». Вариант «Нет» оставляет интерфейс нейтральным.
accent-none = Нет
accent-custom = Свой
settings-accent-scope = Охват акцентного цвета
settings-accent-scope-help = Элементы интерфейса, использующие акцентный цвет
settings-accent-scope-none = Сначала выберите акцентный цвет
accent-scope-minimal = Минимальный
accent-scope-minimal-help = Фокус клавиатуры и контур рабочей области
accent-scope-standard = Стандартный
accent-scope-standard-help = Также основные кнопки, переключатели и выделение
accent-scope-full = Расширенный
accent-scope-full-help = Также вкладки, варианты выбора, активные строки, ползунки и индикаторы выполнения
overview-structure = Структура
env-production = Рабочее
env-staging = Тестовое
site-facts-envs = Окружения публикации: { $list }
seo-org-add = Описать организацию (название, логотип, контакты) для поисковых систем
seo-org-remove = Удалить описание организации
overview-environments = Окружения
action-close = Закрыть
action-edit = Изменить
action-replace = Заменить
new-site-title = Новый сайт
import-overwrite-title = Заменить существующий сайт?
publish-default-name = Мой хостинг
publish-form-new = Новый SFTP-хостинг
publish-form-edit = Изменить хостинг
publish-form-help = Доступ по SFTP предоставляет ваш хостинг-провайдер (OVH, o2switch, Infomaniak…), обычно он находится в личном кабинете в разделе FTP-SSH.
publish-host-help = Пример: ftp.cluster000.hosting.ovh.net
publish-remote-dir-help = Папка сайта на сервере относительно вашей домашней папки. Пример: www/
publish-password-help = Хранится в системной связке ключей, а не в файлах сайта.
publish-check-help = Подключается к серверу и сравнивает файлы, ничего не отправляя и не удаляя.
publish-connecting = Подключение к серверу…
publish-comparing = Сравнение с файлами в сети…
publish-check-failed = Не удалось проверить. { $message }
publish-delete-title = Удалить «{ $name }»?
publish-delete-body = Сохранённые данные доступа и пароль будут стёрты. Файлы, уже находящиеся в сети, не изменятся.
error-detail = { $detail }
error-update = Сервер обновлений недоступен: { $detail }
error-no-desktop = Это действие доступно только в приложении Revel для компьютера.
error-drop-folder-only = Перетащите папку сайта, а не файлы.
error-drop-folder-unknown = Папка «{ $name }» не найдена: выберите её через «Открыть сайт».
error-site-thumb = Не удалось снять изображение главной страницы
error-icon-not-svg = Этот файл не является рисунком SVG.
error-project = Не удалось прочитать сайт: { $detail }
error-import = Не удалось импортировать: { $detail }
error-io = Ошибка чтения или записи: { $detail }
error-already-imported = В этой папке уже есть сайт Revel.
error-not-eleventy = Эта папка не является проектом Eleventy (нет файла eleventy.config.js или .eleventy.js).
error-unknown-profile = Этого хостинга больше нет. Перезагрузите страницу «Публикация».
error-missing-secret = Пароль не найден в связке ключей: откройте настройки хостинга и введите его заново.
error-keyring = Системная связка ключей недоступна: { $detail }
error-blocked = Публикация заблокирована: сначала исправьте перечисленные замечания.
error-not-built = Сайт для этого окружения ещё не создан. Создайте его, затем откройте предпросмотр.
error-sftp-connect = Не удаётся подключиться к { $host }. Проверьте адрес сервера, порт и подключение к интернету. ({ $detail })
error-sftp-timeout = { $host } не отвечает. Проверьте адрес сервера и порт; у o2switch сначала разрешите IP-адрес этого компьютера (cPanel › «Autorisation SSH»).
error-no-previous = Для этого хостинга нет сохранённой предыдущей публикации.
error-publish-checks = { $count ->
    [one] Перед публикацией нужно проверить { $count } пункт.
    [few] Перед публикацией нужно проверить { $count } пункта.
    [many] Перед публикацией нужно проверить { $count } пунктов.
   *[other] Перед публикацией нужно проверить { $count } пункта.
}
error-sftp-host-key = Ключ сервера изменился с момента последнего подключения. Причиной может быть смена настроек у хостинг-провайдера или попытка перехвата: прежде чем продолжить, уточните это у провайдера.
error-sftp-auth = Сервер отклонил логин или пароль.
error-sftp = Сервер вернул ошибку: { $detail }
error-sftp-remote-dir = Недопустимая папка на сервере: «{ $dir }». Укажите относительный путь, например www/
error-sftp-file-name = Неподдерживаемое имя файла: { $name }
publish-fix = Изменить хостинг
publish-kind = Тип хостинга
publish-kind-sftp = SFTP-сервер
publish-kind-github = GitHub Pages
publish-form-new-github = Новый хостинг GitHub Pages
publish-form-help-github = GitHub Pages бесплатно раздаёт сайт из ветки репозитория GitHub. Revel отправляет туда созданный сайт и при необходимости включает GitHub Pages.
publish-repo = Репозиторий
publish-repo-help = Владелец и название репозитория. Пример: matv-studio/my-site
publish-branch = Ветка
publish-branch-help = Ветка, отведённая под созданный сайт; при необходимости будет создана. Не используйте ветку с исходниками.
publish-domain = Собственный домен
publish-domain-help = Необязательно. Пример: www.example.com (его нужно также указать у регистратора домена). Если пусто: адрес вида owner.github.io/repository/
publish-fetching = Загрузка онлайн-версии…
publish-preparing = Подготовка файлов: { $done } / { $total }
publish-open-site = Посмотреть сайт
publish-pages-enabled = GitHub Pages только что включён для этого репозитория: сайт станет доступен в течение нескольких минут.
github-account = Аккаунт GitHub
github-connected = Подключено к GitHub: { $login }
github-signed-out = Аккаунт GitHub не подключён.
github-connect = Подключиться к GitHub
github-disconnect = Отключить
github-code-help = На github.com введите этот код, чтобы разрешить доступ Revel:
github-open = Скопировать код и открыть github.com
github-copy = Скопировать код
github-waiting = Ожидаем вашего подтверждения на github.com… Окно обновится автоматически.
settings-translate = Автоматический перевод
settings-translate-help = Языки, которые переводятся на этом компьютере, без подключения к интернету. Язык скачивается один раз с Hugging Face (модели OPUS-MT Хельсинкского университета); между двумя языками перевод идёт через английский.
settings-translate-add = Добавить
settings-translate-add-help = Входит в состав Revel: добавляется без скачивания
settings-translate-get = Скачать
settings-translate-get-help = Нужно скачать { $size } (один раз)
settings-translate-remove = Удалить язык
settings-translate-remove-title = Убрать «{ $name }» из автоматического перевода?
settings-translate-preparing = Подготовка…
settings-appearance = Внешний вид
settings-projects = Папка проектов
settings-projects-help = Новые сайты создаются здесь, каждый в своей папке. По умолчанию: папка Projects рядом с Revel, если в неё можно писать, иначе Documents\Revel.
settings-projects-reset = Вернуть папку по умолчанию
settings-accounts = Аккаунты
settings-github-help = Нужен для публикации на GitHub Pages. Revel запрашивает доступ к вашим репозиториям; токен хранится в системной связке ключей.
settings-author = Ваше имя
settings-author-help = Имя, которое записывается в версиях, сохранённых с этого компьютера, чтобы при совместной работе было видно, кто что изменил. Если имя не указано, используется имя из Git, а при его отсутствии аккаунт GitHub.
settings-author-placeholder = Имя Фамилия
error-github-signed-out = Аккаунт GitHub не подключён, или подключение истекло. Подключитесь к GitHub и повторите попытку.
error-github-repo = Репозиторий GitHub не найден: «{ $repo }». Проверьте название (владелец/репозиторий) и то, что у подключённого аккаунта есть к нему доступ.
error-github-no-push = У подключённого аккаунта GitHub нет прав на запись в репозиторий { $repo }.
error-github-cancelled = Подключение к GitHub отменено.
error-github-code-expired = Срок действия кода истёк до того, как его ввели. Начните подключение заново.
error-github-denied = Доступ отклонён на github.com.
error-github-app = Приложение Revel неправильно настроено на GitHub ({ $detail }).
error-github-api = GitHub вернул ошибку ({ $status }): { $detail }
error-git-rejected = Репозиторий отклонил отправку: { $detail }. Возможно, ветку изменили в другом месте; опубликуйте ещё раз.
error-git = Ошибка Git: { $detail }
error-network = Не удаётся подключиться. Проверьте подключение к интернету. ({ $detail })
error-edit-not-found = Элемент не найден: возможно, его тем временем удалили. ({ $detail })
error-edit-invalid = Это изменение здесь невозможно. ({ $detail })
error-edit-duplicate = Изменение отклонено: идентификатор элемента повторялся бы. ({ $detail })
error-edit-move = Нельзя поместить элемент внутрь самого себя.
error-edit-tag = Недопустимое имя тега: только буквы, цифры и дефисы. ({ $detail })
error-edit-not-open = Этот сайт не открыт в редакторе.
error-font = Шрифт: { $detail }
error-edit-site = Недопустимый адрес: «{ $detail }» (ожидается: https://…)
error-edit-class = Имя класса отклонено: допустимы буквы, цифры и дефисы, первым символом не может быть цифра.
error-edit-class-taken = Такой класс уже существует.
sync-title = Совместная работа
sync-ok = Всё актуально · проверено в { $time }
sync-busy = Получение версий…
sync-off = Автоматическая синхронизация выключена (Настройки › Версии)
sync-offline = Общий репозиторий не отвечает: повторим попытку через минуту
sync-waiting = Версии созданы с обеих сторон: они будут объединены при следующем сохранении версии
sync-paused = { $count ->
    [one] Не удалось завершить синхронизацию: { $count } место изменено с обеих сторон. Проверьте, какую версию оставить, или отложите.
    [few] Не удалось завершить синхронизацию: { $count } места изменены с обеих сторон. Проверьте, какую версию оставить, или отложите.
    [many] Не удалось завершить синхронизацию: { $count } мест изменено с обеих сторон. Проверьте, какую версию оставить, или отложите.
   *[other] Не удалось завершить синхронизацию: { $count } места изменено с обеих сторон. Проверьте, какую версию оставить, или отложите.
}
sync-received = { $count ->
    [one] Получена { $count } версия от { $authors }
    [few] Получено версий: { $count } (от { $authors })
    [many] Получено версий: { $count } (от { $authors })
   *[other] Получено версий: { $count } (от { $authors })
}
sync-last-received = { $count ->
    [one] Получена { $count } версия (от { $authors }, в { $time })
    [few] Получено версий: { $count } (от { $authors }, в { $time })
    [many] Получено версий: { $count } (от { $authors }, в { $time })
   *[other] Получено версий: { $count } (от { $authors }, в { $time })
}
sync-verify = Проверить
sync-later = Позже
sync-now = Синхронизировать
versions-auto = Автоматическая синхронизация
versions-auto-help = Версии других людей приходят сами (без создания версии, при первой паузе); ваши уходят, как только вы сохраняете версию
error-edit-outdated = Это изменение больше нельзя отменить: коллега с тех пор изменил то же место
error-translate-pair = Нет модели перевода для { $from } → { $to }.
error-translate = Не удалось выполнить автоматический перевод: { $detail }
error-versions-conflict = Одни и те же строки изменены в сети и на сайте: разрешите конфликт в разделе «Настройки › Версии» перед публикацией.
error-translate-integrity = Скачанный файл перевода повреждён ({ $detail }): скачайте его ещё раз.
error-translate-cancelled = Скачивание отменено.
error-translate-busy = Скачивание языка уже выполняется.
error-edit-css = CSS отклонён: допускается одно объявление, без точки с запятой и фигурных скобок вне кавычек и без «!important» в значении (для него есть отдельный флажок). ({ $detail })
error-media = Медиафайл не импортирован: { $detail }
error-edit-expr = Недопустимое выражение, ничего не изменено. ({ $detail })
error-edit-path = Адрес отклонён: он должен начинаться с «/» и содержать только строчные буквы без диакритических знаков, цифры и дефисы. ({ $detail })
error-edit-path-taken = Этот адрес уже используется другой страницей. ({ $detail })
error-edit-redirect = Перенаправление отклонено: старый адрес должен начинаться с «/» и отличаться от нового. ({ $detail })
error-edit-in-use = Этот компонент всё ещё используется: сначала отсоедините или удалите его экземпляры. ({ $detail })
error-edit-prop = Имя свойства отклонено: буквы без диакритических знаков, цифры и «_», по одному на компонент. ({ $detail })
error-edit-self-use = Нельзя поместить компонент внутрь самого себя. ({ $detail })
error-edit-locale = Этот язык нельзя добавить, или он не существует (код: код языка, например en или pt-BR, отличный от основного языка). ({ $detail })
error-edit-no-stylesheet = У этого сайта пока нет таблицы стилей.
error-edit-breakpoints = Недопустимые брейкпоинты: каждая ширина может использоваться только один раз и должна быть положительным числом пикселей. ({ $detail })
error-edit-bound = Этот текст взят из файла, который здесь нельзя изменить. ({ $detail })
error-mcp-no-site = В Revel не открыт ни один сайт: откройте сайт или укажите его папку.
error-mcp-disabled = Сервер ИИ-ассистентов выключен: включите его в Revel, Настройки › ИИ-ассистенты.
error-mcp-unreachable = Revel не отвечает: откройте Revel и убедитесь, что сервер ИИ-ассистентов включён.
error-mcp-token = Revel отклонил ключ доступа: скопируйте актуальный ключ из раздела «Настройки › ИИ-ассистенты».

## Panneau de styles (D-20)
styles-count = { $count ->
        [one] { $count } элемент
        [few] { $count } элемента
        [many] { $count } элементов
       *[other] { $count } элемента
    }
styles-origin-here = Задано здесь: { $source }
styles-origin-inherited = Унаследовано от { $source }
styles-origin-none = Значение пришло из другого места (другого класса, родительского элемента или браузера).
styles-label-help = Перетаскивание: изменить значение (Shift ×10, Alt ×0.1). Alt+щелчок: сбросить
styles-unset = —
display-block = Блок
display-flex = Flex
display-grid = Grid
display-none = Скрыт
display-more = Другие режимы отображения
display-inline = Строчный
display-inline-block = Строчно-блочный
display-inline-flex = Строчный Flex
display-inline-grid = Строчный Grid
display-contents = Только содержимое
dir-horizontal = По горизонтали
dir-vertical = По вертикали
dir-reverse = Обратный порядок
align-horizontal = По горизонтали
align-vertical = По вертикали
align-cell = Выравнивание: столбец { $col }, строка { $row }
align-left = Слева
align-center = По центру
align-right = Справа
align-top = Сверху
align-bottom = Снизу
align-stretch = Растянуть
align-baseline = По базовой линии
align-between = Равные промежутки между
align-around = Равные промежутки вокруг
align-evenly = Равномерно
styles-wrap = Перенос
styles-flex-child = Дочерний Flex-элемент
flex-shrink = Сжимать при необходимости
flex-grow = Растягивать, если возможно
flex-none = Не сжимать и не растягивать
styles-spacing-help = Перетащите значение, чтобы изменить его (Alt: и с противоположной стороны, Shift: со всех четырёх сторон), щёлкните, чтобы ввести.
styles-side-help = перетащите или щёлкните
overflow-visible = Видимое
overflow-hidden = Скрытое
overflow-scroll = Прокрутка
overflow-auto = Авто
styles-ratio = Соотношение сторон
ratio-auto = Свободное
ratio-square = Квадрат (1:1)
ratio-wide = Широкое (16:9)
ratio-classic = Классическое (4:3)
ratio-landscape = Альбомное (3:2)
ratio-portrait = Книжное (2:3)
ratio-tall = Вертикальное (9:16)
styles-fit = Вписывание
fit-cover = Заполнить (с обрезкой)
fit-contain = Вписать (показать целиком)
fit-fill = Растянуть
fit-none = Реальный размер
fit-scale-down = Уменьшить при необходимости
unit-menu = Единица
nbox-fluid = Плавное
nbox-fluid-help = Плавное значение: между минимумом и максимумом в зависимости от ширины экрана
nbox-fn-min = Мин.
nbox-fn-ideal = Идеал
nbox-fn-max = Макс.
nbox-fn-value = Значение { $n }
nbox-fixed = Фиксированное значение
nbox-fixed-help = Заменить одним значением
nbox-vars = Переменные сайта
nbox-detach = Отсоединить от переменной
nbox-detach-help = Заменить текущим значением переменной
unit-px = пиксели, фиксированный размер
unit-percent = проценты от родителя
unit-vw = проценты от ширины экрана
unit-vh = проценты от высоты экрана
unit-em = относительно размера текста
unit-rem = относительно размера текста страницы
unit-auto = автоматически
unit-fit = по содержимому
unit-none = без ограничения
unit-normal = обычное
unit-times = кратно размеру текста
position-static = Статическое
position-relative = Относительное
position-absolute = Абсолютное
position-fixed = Фиксированное
position-sticky = Прилипающее
position-static-help = Следует потоку страницы.
position-relative-help = Смещается от своего обычного места; служит точкой отсчёта для вложенных абсолютных элементов.
position-absolute-help = Размещается относительно ближайшего позиционированного родителя, вне потока страницы.
position-fixed-help = Размещается относительно окна: остаётся на виду при прокрутке.
position-sticky-help = Прокручивается как обычно, затем прилипает к указанному краю.
styles-offsets = Смещения
styles-preset = Пресет
preset-choose = Выбрать…
preset-full = Весь родитель
preset-top = Верхний край
preset-bottom = Нижний край
preset-left = Левый край
preset-right = Правый край
preset-top-left = Левый верхний угол
preset-top-right = Правый верхний угол
preset-bottom-left = Левый нижний угол
preset-bottom-right = Правый нижний угол
font-system = Системный шрифт
font-serif = С засечками
font-mono = Моноширинный
weight-variable = Переменная
weight-not-loaded = { $name } (не загружен)
pos-top-left = Слева сверху
pos-top = Сверху
pos-top-right = Справа сверху
pos-left = Слева
pos-center = По центру
pos-right = Справа
pos-bottom-left = Слева снизу
pos-bottom = Снизу
pos-bottom-right = Справа снизу
weight-100 = Тонкий
weight-200 = Сверхлёгкий
weight-300 = Лёгкий
weight-400 = Обычный
weight-500 = Средний
weight-600 = Полужирный
weight-700 = Жирный
weight-800 = Сверхжирный
weight-900 = Чёрный
text-left = Слева
text-center = По центру
text-right = Справа
text-justify = По ширине
styles-text-style = Начертание
text-italic = Курсив
text-underline = Подчёркивание
text-strike = Зачёркивание
case-none = Как введено
case-upper = ВЕРХНИЙ РЕГИСТР
case-lower = нижний регистр
case-capitalize = Каждое Слово С Заглавной
bg-add = Добавить изображение или градиент
bg-none = Нет
bg-image = Изображение
bg-linear = Линейный градиент
bg-radial = Радиальный градиент
bg-remove = Удалить этот слой
bg-image-change = Изменить
bg-angle = Угол
bg-angle-help = Перетащите, чтобы повернуть градиент (Shift: шаг 15°)
bg-shape = Форма
bg-circle = Круг
bg-ellipse = Эллипс
bg-center = Центр
bg-center-x = Центр по горизонтали
bg-center-y = Центр по вертикали
bg-stops = Цвета
bg-stops-help = Перетащите цвет, чтобы переместить его, щёлкните по полосе, чтобы добавить новый
bg-stop = Цвет градиента
bg-stop-color = Цвет
bg-stop-position = Положение цвета
bg-stop-remove = Удалить этот цвет
bg-cover = Заполнить
bg-contain = Вписать
bg-auto = Реальный размер
styles-background-repeat = Повтор
corner-independent = Независимые углы
corner-top-left = Левый верхний
corner-top-right = Правый верхний
corner-bottom-left = Левый нижний
corner-bottom-right = Правый нижний
styles-border-sides = Стороны
side-all = Все стороны
side-top = Сверху
side-right = Справа
side-bottom = Снизу
side-left = Слева
border-none = Нет
border-solid = Сплошная
border-dashed = Штриховая
border-dotted = Пунктирная
shadow-none = Без тени
shadow-inherited = Унаследованная тень
shadow-add = Добавить тень
shadow-remove = Удалить тень
shadow-type = Тип тени
shadow-drop = Внешняя тень
shadow-inner = Внутренняя тень
shadow-x = Смещение по горизонтали
shadow-y = Смещение по вертикали
shadow-blur = Размытие
shadow-spread = Растяжение
shadow-color = Цвет тени
styles-cursor = Курсор
cursor-auto = Автоматически
cursor-default = Стрелка
cursor-pointer = Рука (ссылка)
cursor-text = Текст
cursor-move = Перемещение
cursor-grab = Захват
cursor-not-allowed = Запрещено
cursor-help = Справка
cursor-none = Скрыт
color-open = Выбрать цвет
color-plane = Насыщенность и яркость
color-hue = Оттенок
color-opacity = Непрозрачность
color-eyedropper = Взять цвет с экрана
color-model = Цветовая модель
color-rgb = RGB
color-hsl = HSL
color-hsb = HSB
color-none = Нет
color-site = Цвета сайта
color-site-add = Добавить в цвета сайта
color-site-name = Название цвета
color-site-default = Цвет { $n }
color-site-create = Добавить
colors-title = Цвета
colors-help = Цвета сайта, используемые в стилях. При включённой тёмной теме у каждого цвета есть ещё и значение для посетителей, чьё устройство использует тёмное оформление.
colors-name = Название
colors-light = Светлая
colors-dark = Тёмная
colors-none = Нет цветов сайта
colors-dark-mode = Тёмная тема
colors-dark-help = Сайт следует светлой или тёмной теме устройства посетителя
colors-dark-off-title = Удалить цвета тёмной темы?
colors-remove-title = Удалить цвет «{ $name }»?
colors-remove-used = Используется в стилях (раз: { $count }): эти стили лишатся цвета.
styles-contrast = Контраст с фоном: { $ratio }:1, { $level }
styles-contrast-aaa = AAA
styles-contrast-aa = AA
styles-contrast-large = AA только для крупного текста
styles-contrast-low = слишком низкий
unit-auto-s = Авто
unit-fit-s = По содерж.
unit-none-s = Нет
unit-normal-s = Обычное
styles-label-help-reset = Alt+щелчок: сбросить
styles-margin-short = Внешний отступ
styles-padding-short = Внутренний отступ
styles-combo = Составные классы: стиль применяется только к элементам, у которых есть все эти классы
styles-class-remove = Убрать этот класс у элемента
styles-class-first = Добавить класс…
styles-new = Новый
editor-bp-desktop-short = Компьютер

## Espace de travail (D-21)
panel-pages = Страницы
panel-cms = CMS
panel-components = Компоненты
panel-assets = Ресурсы
panel-classes = Классы
classes-search = Поиск классов
classes-count = { $count ->
    [one] { $count } класс
    [few] { $count } класса
    [many] { $count } классов
   *[other] { $count } класса
}
classes-none = Нет классов
classes-no-match = Класс не найден.
classes-unused = не используется
classes-unused-note = Ни у одного элемента нет этого класса
classes-unused-filter = Только неиспользуемые классы
classes-clean = { $count ->
    [0] Нет неиспользуемых классов
    [one] Удалить неиспользуемый класс
    [few] Удалить { $count } неиспользуемых класса
    [many] Удалить { $count } неиспользуемых классов
   *[other] Удалить { $count } неиспользуемого класса
}
classes-clean-title = { $count ->
    [one] Удалить неиспользуемый класс?
    [few] Удалить { $count } неиспользуемых класса?
    [many] Удалить { $count } неиспользуемых классов?
   *[other] Удалить { $count } неиспользуемого класса?
}
classes-row-title = { $uses ->
    [0] Нет элементов
    [one] { $uses } элемент
    [few] { $uses } элемента
    [many] { $uses } элементов
   *[other] { $uses } элемента
} · { $count ->
    [0] нет стилей
    [one] { $count } стиль
    [few] { $count } стиля
    [many] { $count } стилей
   *[other] { $count } стиля
}
classes-go = Перейти к элементу
classes-in-layout = Шаблон { $name }
classes-rename = Переименовать
classes-rename-title = Переименовать «{ $name }»
classes-rename-label = Новое имя (везде: стили, элементы, взаимодействия)
classes-remove = Удалить
classes-remove-title = Удалить класс «{ $name }»?
classes-remove-body = { $uses ->
    [one] Будет убран у { $uses } элемента
    [few] Будет убран у { $uses } элементов
    [many] Будет убран у { $uses } элементов
   *[other] Будет убран у { $uses } элемента
}; { $count ->
    [0] удалять стили не придётся.
    [one] будет удалён { $count } стиль.
    [few] будет удалено { $count } стиля.
    [many] будет удалено { $count } стилей.
   *[other] будет удалено { $count } стиля.
}
classes-remove-unused-body = { $count ->
    [0] Ни у одного элемента его нет, и стилей у него нет.
    [one] Ни у одного элемента его нет; будет удалён { $count } стиль.
    [few] Ни у одного элемента его нет; будет удалено { $count } стиля.
    [many] Ни у одного элемента его нет; будет удалено { $count } стилей.
   *[other] Ни у одного элемента его нет; будет удалено { $count } стиля.
}
styles-class-title = { $name }: двойной щелчок переименует везде
panel-interactions = Взаимодействия
anim-appear = Появление
anim-appear-help = Когда элемент попадает в область видимости или загружается страница
anim-text = Текст
anim-text-help = Буквы, слова или строки анимируются одна за другой
anim-hover = Наведение
anim-hover-help = Пока указатель находится над элементом (на компьютерах)
anim-press = Нажатие
anim-press-help = Во время щелчка или касания
anim-loop = Цикл
anim-loop-help = Непрерывно, приостанавливается, когда элемент за пределами экрана
anim-scroll = Прокрутка
anim-scroll-help = Следует за положением элемента при прокрутке страницы
anim-active = Становится активным
anim-active-help = Каждый раз, когда элемент или содержащая его вкладка, фильтр или переключатель становится активным
anim-add = Добавить анимацию: { $name }
anim-remove = Удалить анимацию
anim-preview = Предпросмотр на холсте
anim-wrap = Обернуть для анимации
anim-wrap-help = Изображение анимируется через содержащий его блок: вокруг него создаётся такой блок и выделяется
anim-p-fade = Проявление
anim-p-up = Подъём
anim-p-down = Падение
anim-p-left = Слева
anim-p-right = Справа
anim-p-zoom-in = Приближение
anim-p-zoom-out = Отдаление
anim-p-blur = Размытие
anim-p-pop = Выскакивание
anim-p-flip = Переворот
anim-p-rotate = Поворот
anim-p-reveal = Раскрытие
anim-p-typewriter = Печатная машинка
anim-p-grow = Рост
anim-p-shrink = Сжатие
anim-p-lift = Приподнимание
anim-p-shadow = Тень
anim-p-dim = Затемнение
anim-p-tilt = Наклон
anim-p-brighten = Осветление
anim-p-underline = Подчёркивание
anim-p-press = Нажатие
anim-p-squish = Сплющивание
anim-p-push = Вдавливание
anim-p-spin = Кручение
anim-p-pulse = Пульсация
anim-p-float = Парение
anim-p-swing = Качание
anim-p-shake = Тряска
anim-p-blink = Мигание
anim-p-bounce = Подпрыгивание
anim-p-heartbeat = Сердцебиение
anim-p-ticker = Бегущая строка
anim-p-parallax = Параллакс
anim-p-fade-in = Постепенное появление
anim-p-zoom-in-scroll = Приближение
anim-p-rotate-scroll = Вращение
anim-p-slide-x = Сдвиг
anim-p-fade-out = Исчезновение
anim-p-jump = Прыжок
anim-p-flash = Вспышка
anim-d-appear-fade = Непрозрачность от 0 до 100%
anim-d-appear-up = Появляется снизу
anim-d-appear-down = Появляется сверху
anim-d-appear-left = Появляется слева
anim-d-appear-right = Появляется справа
anim-d-appear-zoom-in = Увеличивается с 90% до 100%
anim-d-appear-zoom-out = Уменьшается с 110% до 100%
anim-d-appear-blur = Появляется в фокусе (размытие 12 px)
anim-d-appear-pop = Увеличивается с 50% до 100% с небольшим перелётом
anim-d-appear-flip = Переворачивается вперёд в 3D
anim-d-appear-rotate = Слегка поворачивается при подъёме
anim-d-appear-reveal = Раскрывается снизу вверх, как занавес
anim-d-text-fade = Каждая часть проявляется
anim-d-text-up = Каждая часть поднимается, проявляясь
anim-d-text-blur = Каждая часть появляется в фокусе
anim-d-text-pop = Каждая часть увеличивается с перелётом
anim-d-text-flip = Каждая часть переворачивается в 3D
anim-d-text-typewriter = Буквы появляются по одной
anim-d-hover-grow = Масштаб 105%
anim-d-hover-shrink = Масштаб 95%
anim-d-hover-lift = Поднимается на 6 px с тенью
anim-d-hover-shadow = Более сильная внешняя тень
anim-d-hover-dim = Непрозрачность 70%
anim-d-hover-tilt = Поворот на 3°
anim-d-hover-brighten = Яркость 115%
anim-d-hover-underline = Под текстом рисуется линия
anim-d-press-press = Масштаб 95%
anim-d-press-squish = Сплющивается, затем возвращается
anim-d-press-push = Смещается вниз на 2 px
anim-d-loop-spin = Равномерный полный оборот
anim-d-loop-pulse = Увеличивается до 106% и возвращается
anim-d-loop-float = Движется вверх и вниз на 10 px
anim-d-loop-swing = Качается на ±6°
anim-d-loop-shake = Трясётся влево и вправо
anim-d-loop-blink = Непрозрачность от 100% до 30%
anim-d-loop-bounce = Подпрыгивает и приземляется
anim-d-loop-heartbeat = Двойное сердцебиение
anim-d-loop-ticker = Содержимое, бесконечно прокручивающееся (логотипы, слова)
anim-d-scroll-parallax = Движется медленнее страницы (80 px)
anim-d-scroll-fade-in = Проявляется при входе в окно
anim-d-scroll-zoom-in = Увеличивается с 85% до 100% при входе
anim-d-scroll-rotate = Поворачивается при пересечении окна (±10°)
anim-d-scroll-slide-x = Пересекает окно справа налево (120 px)
anim-d-scroll-fade-out = Исчезает при выходе через верхний край
anim-d-active-pop = Увеличивается до 150% и возвращается
anim-d-active-jump = Подпрыгивает на 12 px и приземляется
anim-d-active-shake = Трясётся влево и вправо
anim-d-active-flash = Непрозрачность от 100% до 30% и обратно
anim-d-active-spin = Делает полный оборот
anim-trigger = Запуск
anim-trigger-view = При появлении в окне
anim-trigger-load = При загрузке страницы
anim-trigger-view-help = Когда элемент попадает в область видимости
anim-trigger-load-help = Сразу после загрузки страницы
anim-dur = Длительность
anim-dur-help = Длительность анимации в секундах
anim-dur-loop-help = Длительность одного цикла в секундах
anim-dur-text-help = Длительность для каждой части в секундах
anim-delay = Задержка
anim-delay-help = Ожидание перед запуском в секундах
anim-ease = Кривая
anim-ease-smooth = Плавная
anim-ease-bounce = С отскоком
anim-ease-ease = Равномерная
anim-ease-linear = Линейная
anim-dist = Расстояние
anim-dist-help = Расстояние, которое элемент проходит при появлении, в пикселях
anim-stagger = Каскад
anim-stagger-help = Вложенные элементы анимируются один за другим: время между двумя в секундах (0: весь элемент целиком)
anim-stagger-text-help = Время между двумя частями в секундах
anim-replay = Повтор
anim-replay-help = Проигрывается заново при каждом появлении в окне
anim-split = Разбивка
anim-split-chars = Буквы
anim-split-words = Слова
anim-split-lines = Строки
anim-amount = Интенсивность
anim-amount-help = Сила эффекта в процентах от предусмотренного значения
anim-mirror = Туда и обратно
anim-mirror-help = Каждый цикл идёт в обратную сторону, а не начинается заново
anim-dir = Направление
anim-dir-left = Влево
anim-dir-right = Вправо
anim-pause = Пауза при наведении
panel-variables = Переменные
variables-search = Поиск переменных
variables-none = На этой странице нет переменных
variables-no-match = Подходящих переменных нет
variables-owner-layout = Макет { $name }
variables-owner-component = Компонент { $name }
variables-owner-page = Страница { $name }
variables-value-empty = пусто
variables-value-list = { $count ->
        [0] пустой список
        [one] список · { $count } элемент
        [few] список · { $count } элемента
        [many] список · { $count } элементов
       *[other] список · { $count } элемента
    }
variables-value-object = { $count ->
        [0] пустой объект
        [one] объект · { $count } ключ
        [few] объект · { $count } ключа
        [many] объект · { $count } ключей
       *[other] объект · { $count } ключа
    }
variables-varies = При каждом проходе значение разное: показано первое
variables-not-run = На этой странице не вычисляется (условие не выполнено)
variables-block = { $name }: захваченное содержимое HTML (блок)
variables-edit = Изменить выражение
variables-copy-name = Скопировать имя
variables-expression = Выражение
planned-step = запланировано на этап { $step }
planned-edit = Редактирование: этап { $step }
pages-search = Поиск страниц
pages-count = { $count ->
        [one] { $count } страница
        [few] { $count } страницы
        [many] { $count } страниц
       *[other] { $count } страницы
    }
pages-new = Новая страница
pages-collection-none = Записи без страницы: у каждой записи может быть своя страница на основе общего шаблона
pages-collection-create = Создать страницы записей
pages-settings = Настройки страницы
pages-name = Название страницы
pages-path = Адрес
pages-home-path = Адрес главной страницы не меняется
pages-redirect = Перенаправлять со старого адреса
pages-redirect-help = Постоянное перенаправление (301) со старого адреса на новый: существующие ссылки и позиции в поиске сохраняются
pages-draft = Черновик
pages-draft-help = Страница-черновик остаётся доступной для редактирования, но не публикуется
pages-home-draft = Главная страница всегда публикуется
pages-draft-tag = черновик
pages-publish = Опубликовать страницу
pages-to-draft = Перенести в черновики
pages-seo = Поисковые системы
template-settings = Настройки шаблона
template-settings-help = Для каждой страницы коллекции; страница может изменить их в своих настройках.
pages-og-type = Тип при отправке в соцсети
pages-og-type-help = Что страница сообщает социальным сетям (og:type): обычная страница сайта или статья с датой.
pages-og-type-auto = Автоматически
pages-og-type-website = Страница
pages-og-type-article = Статья
pages-date = Дата публикации
pages-look = Внешний вид
pages-body-class = Класс тела страницы
pages-body-class-help = Классы, добавляемые к тегу <body> страницы (тема, особая раскладка)
pages-body-class-add = Добавить класс
pages-theme = Тема
pages-theme-help = Светлая или тёмная тема этой страницы (атрибут data-theme)
pages-theme-site = Как на сайте
pages-theme-light = Светлая
pages-theme-dark = Тёмная
pages-file = Содержимое файла
pages-file-help = Серверный файл, записывается как есть (.htaccess, robots.txt, PHP). Теги шаблона обрабатываются: значение {"{{"} site.url {"}}"}, условие {"{%"} if env.staging {"%}"}…{"{%"} endif {"%}"}, список {"{%"} for e in collections.jobs {"%}"}…{"{%"} endfor {"%}"}.
pages-file-computed = Содержит элементы, изображения или компоненты: настраивается в слоях, а не здесь.
pages-file-invalid = Текст не сохранён: { $error }
pages-file-tags = Теги шаблона обрабатываются при публикации: значения между {"{{"} {"}}"}, условия и списки между {"{%"} {"%}"} (окружение публикации: env.staging, записи: collections.name).
pages-file-tags-none = Записывается как есть. В него можно вставить значение сайта ({"{{"} site.url {"}}"}), условие по окружению ({"{%"} if env.staging {"%}"}…{"{%"} endif {"%}"}) или список записей ({"{%"} for e in collections.name {"%}"}…{"{%"} endfor {"%}"}).
pages-seo-title = Заголовок в результатах поиска
pages-seo-description = Описание
pages-seo-image = Изображение для соцсетей
pages-index = Видна поисковым системам
pages-index-help = Выключено: добавляется тег «noindex», а страница убирается из карты сайта
pages-seo-data = Хранится в файле данных { $file } (его также можно изменить в Pages CMS)
pages-seo-computed = Вычисляется шаблоном: изменить можно в логике сайта
pages-serp = Предпросмотр результата поиска
pages-duplicate = Дублировать
pages-copy-word = копия
pages-delete = Удалить
pages-delete-title = Удалить «{ $name }»?
pages-delete-body = Страница и её содержимое будут удалены с сайта. Ctrl+Z вернёт её.
pages-home-delete = Главную страницу нельзя удалить
pages-move-root = Переместить в корень
pages-new-here = Новая страница здесь
pages-folder-not-empty = Удалить (папка должна быть пустой)
action-back = Назад
pages-toggle = Развернуть или свернуть
design-template = Шаблон · { $collection }
design-template-help = Шаблон коллекции (макет «{ $layout }»): изменения применяются ко всем её записям, кроме тех, у которых собственный дизайн
design-no-template = Записи без страницы
design-own = Собственный дизайн
design-own-help = У этой записи собственный дизайн: изменения шаблона к ней больше не применяются
design-detach = Создать собственный дизайн
design-detach-help = Создать дизайн только для этой записи (копию шаблона, который останется без изменений)
design-attach = Вернуть к шаблону
design-orphan-help = Собственный дизайн, запись которого больше не существует
design-orphan-delete = Удалить этот дизайн
design-field-label = Собственный дизайн (Revel)
cms-none = В этом сайте нет коллекций.
cms-entry-new = Новая запись
cms-collection-new = Новая коллекция
component-make = Создать компонент
component-make-name = Название компонента
component-make-confirm = Создать
component-make-none = Сначала выберите элемент, который нужно превратить в компонент.
component-missing = Компонент не найден: { $name }
component-edit = Изменить компонент
slot-title = Содержимое
slot-help = Содержимое, помещённое в слот компонента: собственное содержимое компонента или содержимое, специфичное для этого экземпляра
slot-default = Содержимое компонента
slot-own = Содержимое этого экземпляра
slot-edit = Изменить здесь
slot-edit-help = Скопировать содержимое компонента в этот экземпляр, чтобы изменить его только здесь (тексты, добавленные или удалённые элементы)
slot-reset = Вернуть содержимое компонента
slot-reset-help = Убрать собственное содержимое экземпляра: снова будет показано содержимое компонента
inspector-slot = Слот содержимого
inspector-slot-help = Каждый экземпляр компонента может заменить содержимое этого элемента своим (инструменты карточки, пункты списка); если собственного содержимого нет, показывается это
component-detach = Отсоединить
component-detach-help = Заменяет этот экземпляр копией его содержимого, которую можно редактировать отдельно
component-props = Свойства
component-props-empty = Свяжите текст, изображение, ссылку или видимость элемента со свойством, чтобы настраивать каждый экземпляр отдельно.
component-prop-inverted = { $label } (инвертировано)
layout-props = Свойства макета
layout-props-empty = Свяжите текст, изображение, ссылку или видимость элемента макета со свойством, чтобы настраивать его для каждой страницы отдельно.
layout-props-help = Значения, выбранные для этой страницы; элементы макета (шапка, подвал…) связываются с ними на вкладке «Стили». Добавленное свойство действует на каждой странице с этим макетом.
component-prop-add = Добавить свойство
component-prop-remove = Удалить свойство (у его элементов останется значение по умолчанию)
component-prop-default = Значение по умолчанию
component-prop-label = Название свойства
component-prop-reset = Вернуть значение по умолчанию
component-prop-computed = { $label } (вычисляется)
prop-kind-text = Текст
prop-kind-image = Изображение
prop-kind-link = Ссылка
prop-kind-switch = Видимость
prop-kind-list = Список
component-bindings = Связано со свойствами
component-bind = Связать со свойством
component-bind-new = Новое свойство «{ $label }»
component-bind-field = Связать с полем записи
component-unbind = Разорвать связь (будет записано значение по умолчанию)
component-field-url = Адрес страницы записи
component-field-fallback = { $field }, иначе { $other }
component-bind-text = Текст
component-bind-image = Изображение
component-bind-link = Ссылка
component-bind-visibility = Видимость
component-done = Готово
component-rename = Переименовать
component-delete = Удалить компонент
component-delete-title = Удалить компонент «{ $name }»?
component-empty = В этом компоненте нет элементов для редактирования.
components-none = Компонентов нет: выберите элемент, затем нажмите «+», чтобы создать компонент.
components-count = { $count ->
        [0] нет компонентов
        [one] { $count } компонент
        [few] { $count } компонента
        [many] { $count } компонентов
       *[other] { $count } компонента
    }
components-search = Поиск компонентов
components-uses = { $count ->
        [one] Используется { $count } раз
       *[other] Используется { $count } раз
    }
components-new = Создать компонент из выделенного
assets-search = Поиск ресурсов
assets-import = Импортировать ресурсы
library-new-folder = Новая папка
library-new-subfolder = Новая вложенная папка
library-folder-name = Название папки
library-open = Открыть
library-rename = Переименовать
library-delete-folder = Удалить папку
library-move-to = Переместить в «{ $folder }»
library-move-out = Убрать из папки
media-type-filter = Тип медиа
media-all-types = Все типы
media-kind-image = Изображения
media-kind-video = Видео
media-kind-audio = Аудио
media-kind-document = Документы
media-kind-font = Шрифты
media-kind-animation = Анимации
media-kind-other = Другое
media-sort = Сортировка
media-sort-recent = Сначала новые
media-sort-oldest = Сначала старые
media-sort-name = По названию (А → Я)
media-sort-name-desc = По названию (Я → А)
media-sort-type = По типу
media-sort-size = По размеру
media-thumb-size = Размер миниатюр
media-folder-expand = Развернуть на месте
media-details = Подробности
media-insert = Поместить на страницу
media-insert-short = Поместить
media-url = Адрес
media-size = Размер
media-alt = Альтернативный текст по умолчанию
media-alt-help = Что изображено, для программ экранного доступа
media-replace = Заменить
media-replace-help = При замене адрес сохраняется: все страницы, где используется этот файл, обновятся. Старый файл сохраняется в .revel/corbeille.
media-copy-url = Скопировать адрес
media-none = В папках сайта нет медиафайлов
media-no-match = Медиафайлы не найдены
media-online = { $count ->
    [one] { $count } изображение из интернета
    [few] { $count } изображения из интернета
    [many] { $count } изображений из интернета
   *[other] { $count } изображения из интернета
}
media-online-help = Изображения, подключённые с другого сайта (бесплатные фото шаблона). После копирования в сайт они больше не зависят от того сайта и оптимизируются так же, как ваши медиафайлы.
media-online-copy = Скачать изображения в медиа сайта
media-online-copying = Копирование…
media-count = { $count ->
        [0] нет медиафайлов
        [one] { $count } медиафайл
        [few] { $count } медиафайла
        [many] { $count } медиафайлов
       *[other] { $count } медиафайла
    }
picker-image-title = Выберите изображение
picker-media-title = Выберите файл
picker-folder = Папка
picker-all-folders = Все папки
picker-choose-image = Выбрать изображение из медиа
picker-choose-media = Выбрать файл из медиа
picker-add-many = { $count ->
    [0] Добавить
    [one] Добавить { $count } элемент
    [few] Добавить { $count } элемента
    [many] Добавить { $count } элементов
   *[other] Добавить { $count } элемента
}
picker-count-images = { $count ->
        [0] нет изображений
        [one] { $count } изображение
        [few] { $count } изображения
        [many] { $count } изображений
       *[other] { $count } изображения
    }
size-kb = { $n } KB
size-mb = { $n } MB
size-gb = { $n } GB
dock-menu = Параметры панели
dock-float = Открепить
dock-close = Закрыть «{ $panel }»
dock-move = Переместить окно
dock-panels = Панели
dock-show-all = Показать панели
dock-hide-all = Скрыть панели
dock-reset = Раскладка по умолчанию
dock-fold = Свернуть колонку
dock-unfold = Развернуть колонку
layers-search = Поиск слоёв
layers-collapse = Свернуть все
layers-parent = Выбрать родителя
editor-zoom-help = Масштаб (Alt+колесо; Shift+1 по размеру окна, Shift+0 100%, Shift+2 по выделению)
editor-zoom-fit-pct = По размеру окна ({ $pct })
editor-zoom-fit-short = Масштаб по размеру окна
editor-zoom-100 = Масштаб 100%
editor-path = Путь к элементу
editor-scrub = Изменить число
key-zoom-fit = Shift+1
key-zoom-100 = Shift+0
key-zoom-wheel = Alt+колесо
key-panels = Ctrl+\
key-alt-click = Alt+щелчок
key-drag-label = Перетащите подпись
home-search = Поиск сайтов
home-view = Вид
home-view-grid = Сетка
home-view-list = Список
action-edit-site = Открыть редактор

# Site views (top bar)
views = Разделы сайта
view-editor = Редактор
view-cms = CMS
view-locales = Локализация
view-settings = Настройки
nav-app-settings = Настройки Revel
preview-help = Показывает сайт таким, каким он будет опубликован, на текущей странице (Shift+щелчок: в новом окне)
site-settings = Настройки сайта
action-open-help = Открывает папку с сайтом Revel
action-import-help = Импортировать существующий сайт: папку со страницами HTML (в том числе экспорт из Webflow), действующий сайт (Framer…), репозиторий Git, файл Figma, экспорт WordPress или проект Eleventy
home-sort = Сортировка
home-sort-recent = Недавние
home-sort-name = По названию
new-site-placeholder = Мой сайт
new-site-lang = Язык сайта
new-site-template = Шаблон
new-site-template-blank = Пустой
new-site-template-blank-help = Одна главная страница, которую нужно заполнить
new-site-more = Ещё
new-site-more-help = Все шаблоны: по профессиям, онлайн-каталог, файл
new-site-template-showcase = Визитка
new-site-template-showcase-help = Главная, «О нас» и «Контакты» из блоков, с меню
new-site-template-blog = Блог
new-site-template-blog-help = Главная, список статей и статьи, которые можно редактировать в CMS
new-site-location = Расположение
new-site-change = Изменить…
new-site-create = Создать сайт
new-site-creating = Создание…
new-site-auto = Как на устройстве
new-site-scheme-help = Светлые цвета, тёмные цвета или оба набора в зависимости от настройки устройства посетителя (позже можно изменить в разделе Настройки › Общие › Цвета)
template-pages = Страниц: { $n }
template-source = Источник шаблонов
template-source-builtin = Встроенные
template-source-catalog = Каталог
template-open-file = Открыть файл
template-open-file-help = Открыть файл шаблона (.revel-template), который вы получили или сохранили
template-file-kind = Шаблон Revel
template-from-file = Файл шаблона
template-catalog-loading = Чтение каталога…
template-catalog-unreachable = Каталог { $repo } не найден
template-catalog-empty = В этом каталоге пока нет шаблонов
template-placeholders = Изображений-заглушек для замены: { $n }
template-placeholders-help = Изображения шаблона заменены онлайн-заглушками: замените их своими в редакторе
template-save-lead = Сохраните сайт как шаблон: лёгкий файл, которым можно поделиться, или каталог GitHub со всеми файлами
template-save-title = Шаблон
template-name = Название шаблона
template-category = Категория
template-description = Описание
template-author = Автор
template-license = Лицензия
template-license-help = Условия повторного использования шаблона (тексты, макет, изображения, отправленные в каталог)
template-license-none = Все права защищены
template-preview = Миниатюра
template-preview-help = Изображение, показываемое в галерее шаблонов (не более 2 MB)
template-preview-none = Цвета сайта
template-target-file = Файл шаблона
template-target-file-what = Лёгкий: значки и шрифты включены, изображения заменены ссылками на заглушки
template-target-file-help = Файл, который можно отправить или сохранить; изображения сайта заменяются онлайн-заглушками того же размера, видео и документы не включаются
template-save-file = Сохранить…
template-saving = Сохранение…
template-target-catalog = Каталог GitHub
template-target-catalog-help = Отправляет шаблон целиком (с изображениями, видео и файлами): добавляется, если у вас есть право записи в репозиторий, иначе предлагается в виде pull request
template-catalog-help = Репозиторий GitHub с каталогом (Настройки › Шаблоны)
template-needs-github = Подключите аккаунт GitHub (Настройки › Профиль и аккаунты)
template-send = Отправить
template-sending = Отправка…
template-saved = Шаблон сохранён · { $size } · изображений-заглушек: { $images } · не включено медиафайлов: { $missing }
template-published = Шаблон добавлен в каталог · файлов: { $files } · { $size }
template-published-pr = Pull request открыт · файлов: { $files } · { $size }
settings-templates = Шаблоны
settings-templates-help = Общий каталог шаблонов на GitHub, который предлагается при создании нового сайта
template-catalog = Каталог
template-catalog-count = Шаблонов: { $n }
template-catalogs = Каталоги
template-catalogs-help = Репозитории GitHub, шаблоны которых показаны в «Новый сайт › Ещё › Каталог»
template-catalogs-none = Нет каталогов
template-catalog-remove = Удалить этот каталог
template-catalog-add = Добавить
template-catalog-add-help = Добавить каталог шаблонов GitHub
template-catalog-add-placeholder = владелец/репозиторий
template-catalog-invalid = Укажите владелец/репозиторий или адрес github.com
template-catalog-missing = Не найден
template-catalog-missing-help = Репозиторий не существует, является закрытым или в нём нет catalog.json
template-catalog-open = Открыть на GitHub
template-catalog-own = Ваш каталог
template-catalog-create = Создать каталог на GitHub
template-catalog-create-help = Создаёт в вашем аккаунте GitHub публичный репозиторий «revel-templates» с файлом catalog.json и выбирает его в качестве каталога
template-catalog-creating = Создание…
error-template-file = Не удалось прочитать файл шаблона ({ $detail })
error-template-version = Шаблон сохранён более новой версией Revel ({ $detail }): обновите Revel
error-template-catalog = Каталог шаблонов недоступен ({ $detail })
error-template-catalog-missing = Шаблона «{ $detail }» нет в каталоге
error-template-publish = Отправка в каталог прервана ({ $detail })
template-cat-all = Все
template-cat-business = Бизнес
template-cat-creative = Творчество
template-cat-commerce = Магазин
template-cat-editorial = Издания
template-cat-food = Еда и напитки
template-cat-events = События
template-cat-travel = Проживание
template-cat-services = Услуги
template-cat-education = Образование
template-cat-community = Сообщества
template-cat-personal = Личные

# Site settings
settings-general = Общие
settings-publish = Публикация
settings-export = Экспорт
settings-seo = SEO
settings-redirects = Перенаправления
settings-fonts = Шрифты
settings-code = Свой код
settings-forms = Формы
settings-versions = Версии
versions-help = Сохранённые версии сайта (репозиторий Git в его папке): сравнивайте, восстанавливайте, синхронизируйте с репозиторием сайта на GitHub (коммиты Pages CMS)
versions-changes = { $n ->
    [one] { $n } изменение
    [few] { $n } изменения
    [many] { $n } изменений
   *[other] { $n } изменения
}
versions-changes-help = Файлы, изменённые с момента последней версии
versions-clean = Всё актуально
versions-sync = Синхронизировать
versions-sync-help = Сохранить изменения, получить версии из сети (Pages CMS), затем отправить версии сайта
versions-sync-account = Подключите аккаунт GitHub в настройках приложения
versions-ahead = Версии к отправке
versions-behind = Версии в сети к получению
versions-save = Сохранить версию
versions-message = Описание
versions-default-message = Версия от { $date }
versions-first = Первая версия
versions-merge-message = Версии, полученные из онлайн-репозитория
versions-init = Включить версии
versions-init-help = Папка сайта станет репозиторием Git: каждая версия сохраняет все её файлы
versions-remote = Репозиторий GitHub
versions-remote-help = HTTPS-адрес репозитория сайта, синхронизируемого с Pages CMS
versions-branch = Ветка
versions-conflict = Одни и те же строки изменены в сети и на сайте: ничего не изменено.
versions-keep-ours = Оставить версию сайта
versions-keep-theirs = Взять версию из сети
versions-resolved-ours = Конфликт разрешён: оставлена версия сайта
versions-resolved-theirs = Конфликт разрешён: оставлена версия из сети
versions-current = Текущие изменения
versions-current-help = Ещё не сохранены
versions-latest = Последняя
versions-files = Изменённые файлы
versions-restore = Восстановить
versions-restore-help = Вернуть файлы этой версии (сначала сохраняется текущее состояние)
versions-restore-title = Восстановить «{ $name }»?
versions-before-restore = Перед восстановлением
versions-restored = Восстановление «{ $name }»
versions-no-files = Ни один файл не изменён
versions-added = Добавлено
versions-deleted = Удалено
versions-modified = Изменено
versions-binary = Двоичный файл
versions-more-lines = Ещё строк: { $n }

# Team work (D-89)
versions-team = Командная работа
versions-team-help = Каждый работает над своей копией сайта, затем синхронизирует её: чужие версии загружаются, ваши отправляются. Изменения в разных местах объединяются; если место изменено с обеих сторон, выбирать вам.
versions-team-title = Работать в команде
versions-mode-solo = Один
versions-mode-solo-help = Версии хранятся в папке сайта на этом компьютере
versions-mode-shared = Общая папка
versions-mode-shared-help = Общие версии в папке локальной сети (NAS, сервер, общий ресурс Windows): без аккаунтов и сторонних сервисов
versions-mode-online = Онлайн-репозиторий
versions-mode-online-help = Общие версии на GitHub, GitLab или другом сервисе Git: доступны отовсюду, содержимое можно править в браузере через Pages CMS
versions-compare-where = Общие версии
versions-compare-where-help = Куда каждый отправляет версии и откуда их получает; на каждом компьютере также хранится полная копия сайта, и с ней можно работать без сети
versions-compare-where-shared = Сетевая папка (NAS, сервер)
versions-compare-where-online = GitHub, GitLab…
versions-compare-accounts = Аккаунты
versions-compare-accounts-help = Что нужно для отправки и получения версий. На GitHub у каждого свой бесплатный аккаунт, приглашённый в репозиторий (Settings › Collaborators, без ограничений): условия GitHub запрещают делиться логином.
versions-compare-accounts-shared = Не нужны, достаточно доступа к папке
versions-compare-accounts-online = Один аккаунт GitHub на человека, приглашённый в репозиторий
versions-compare-remote = Вне офиса
versions-compare-remote-help = Синхронизация не из локальной сети
versions-compare-remote-shared = Нет, только через VPN
versions-compare-remote-online = Да
versions-compare-browser = Содержимое можно править в браузере
versions-compare-browser-help = Pages CMS позволяет править содержимое (статьи, записи…) прямо в онлайн-репозитории, без Revel
versions-compare-browser-shared = Нет
versions-compare-browser-online = Да, через Pages CMS
versions-compare-backup = Копии сайта
versions-compare-backup-help = Места, где хранятся все версии сайта
versions-compare-backup-shared = NAS и каждый компьютер
versions-compare-backup-online = Сеть и каждый компьютер
versions-compare-provider = Провайдер
versions-compare-provider-help = Сторонний сервис, от которого зависит командная работа
versions-compare-provider-shared = Нет
versions-compare-provider-online = GitHub, GitLab…
versions-shared-folder = Общая папка
versions-shared-folder-help = Сетевая папка с общими версиями: Revel создаст в ней «название-сайта.git» и отправит версии. Остальные откроют сайт через «Сайты › Импорт › Git», указав эту папку.
versions-shared-choose = Выбрать…
versions-online-url = Адрес репозитория
versions-online-url-help = HTTPS-адрес репозитория (GitHub, GitLab…); для закрытого репозитория GitHub подключите аккаунт GitHub в настройках
versions-team-on = Включить
versions-team-off-title = Остановить командную работу?
versions-team-off = Остановить
versions-team-copy = Скопировать адрес для команды (Импорт › Git)
versions-team-copied = Адрес скопирован
versions-items = { $n ->
    [one] { $n } место изменено с обеих сторон
    [few] { $n } места изменены с обеих сторон
    [many] { $n } мест изменено с обеих сторон
   *[other] { $n } места изменено с обеих сторон
}
versions-items-help = Остальные изменения объединены. Выберите, какую версию оставить для каждого места, затем примените: результат будет сохранён и отправлен.
versions-pick-ours = Моя
versions-pick-theirs = Полученная
versions-pick-all-ours = Все мои
versions-pick-all-theirs = Все полученные
versions-apply = Применить
versions-removed = Удалено
versions-other-files = Другие файлы
versions-resolved-items = Места, изменённые с обеих сторон, разрешены
versions-area-page = Страница { $name }
versions-area-layout = Макет «{ $name }»
versions-area-component = Компонент «{ $name }»
versions-area-collection = Коллекция «{ $name }»
versions-area-styles = Стили сайта
versions-area-settings = Настройки сайта
versions-area-site = Сайт
versions-area-data = Файл { $name }
time-now = Только что
time-min = { $n } мин назад
time-hours = { $n ->
    [one] { $n } ч назад
   *[other] { $n } ч назад
}
settings-seo-planned = Заголовок, описание и изображение для соцсетей каждой страницы, карта сайта и robots.txt.
settings-forms-planned = Получение данных, отправленных через формы сайта (электронная почта, таблица, внешний сервис).
settings-identity = Основное
settings-site-name = Название
settings-site-url = Публичный адрес
settings-site-lang = Язык
settings-site-url-help = Адрес опубликованного сайта (https://…): ссылки между языками, карта сайта, публикации в соцсетях
settings-site-lang-help = Язык страниц сайта (атрибут lang); остальные языки настраиваются в разделе «Локализация»
settings-typography = Французская типографика
settings-typography-help = Неразрывные пробелы перед : ; ! ? и внутри « », типографский апостроф, в текстах на французском
code-help = Добавляется как есть на каждую опубликованную страницу и в предпросмотр; в редакторе не выполняется
code-head = Head
code-head-help = Перед </head>: метатеги, таблицы стилей, аналитика
code-body = Конец страницы
code-body-help = Перед </body>: скрипты
code-pages = Страницы со своим кодом
code-pages-help = Код, добавляемый после кода сайта, в настройках этих страниц
page-code = Свой код
page-code-help = Добавляется на эту страницу после кода сайта (Настройки › Свой код)
fonts-help = Файлы копируются в сайт и раздаются им самим: посетители не обращаются к серверам Google
fonts-import = Импортировать файлы
fonts-google = Google Fonts
fonts-google-help = Добавить шрифт Google (файлы один раз скачиваются в сайт)
fonts-none = В сайте не объявлено ни одного шрифта
fonts-italic = курсив
fonts-unused = Не используется
fonts-uses = { $n ->
    [one] { $n } правило
    [few] { $n } правила
    [many] { $n } правил
   *[other] { $n } правила
}
fonts-remove = Удалить шрифт
fonts-remove-title = Удалить «{ $name }» из сайта?
fonts-search = Поиск шрифтов
fonts-loading = Загрузка каталога…
fonts-variants = Насыщенность
fonts-add-help = Файлы WOFF2 выбранной насыщенности скачиваются в сайт для языков сайта
fonts-family = Семейство
fonts-variable = Переменный шрифт: все значения насыщенности в диапазоне
settings-site-folder = Папка
redirects-add = Добавить перенаправление
redirects-none = Перенаправлений нет.
redirects-from = Старый адрес
redirects-to = Новый адрес
redirects-help = Старый адрес навсегда перенаправляет (301) на новый. При смене адреса опубликованной страницы перенаправление добавляется автоматически.
redirects-remove = Удалить перенаправление
seo-share = Соцсети и браузер
seo-share-help = Заголовок, описание и изображение для страниц, у которых их нет, значки и цвет для браузера; Revel записывает их на каждую страницу.
seo-site-title = Отображаемое название сайта
seo-site-title-help = Добавляется к заголовку каждой страницы («Страница | Название») и указывается при публикации в соцсетях.
seo-description = Описание по умолчанию
seo-description-help = Для страниц без собственного описания.
seo-share-image = Изображение для соцсетей по умолчанию
seo-share-image-help = Для страниц, у которых нет ни своего изображения для соцсетей, ни обложки (рекомендуется 1200 × 630 px).
seo-theme-color = Цвет браузера
seo-theme-color-help = Цвет адресной строки на телефонах.
seo-favicon = Значок сайта
seo-favicon-help = Файл SVG или PNG, показываемый на вкладке браузера.
seo-favicon-png = Значок PNG
seo-favicon-png-help = Запасной значок для браузеров без поддержки SVG (192 × 192 px).
seo-touch-icon = Значок для экрана «Домой»
seo-touch-icon-help = Значок ярлыка на iPhone и iPad (180 × 180 px).
seo-org = Организация
seo-org-help = Карточка организации, которую читают поисковые системы (структурированные данные schema.org); статьи указывают её как издателя.
seo-org-name = Название
seo-org-alt = Другое название
seo-org-logo = Логотип
seo-org-email = Эл. почта
seo-org-phone = Телефон
seo-org-founded = Год основания
seo-org-street = Адрес
seo-org-postcode = Почтовый индекс
seo-org-city = Город
seo-org-country = Страна (код)
seo-org-social = Социальные сети
seo-org-social-help = Один адрес на строку.
seo-sitemap = Карта сайта (sitemap.xml)
seo-sitemap-lastmod = Даты обновления
seo-sitemap-lastmod-help = Каждый адрес в карте сайта содержит дату публикации страницы или записи (поле «date»), если она есть.
seo-sitemap-help = Создаётся при публикации: все опубликованные страницы, кроме скрытых от поисковых систем.
seo-sitemap-own = У сайта есть собственная страница sitemap.xml: публикуется она.
seo-sitemap-no-url = Не указан адрес сайта (раздел «Общие»): карту сайта создать нельзя.
seo-robots-help = Если пусто: разрешить всё и указать карту сайта.
seo-robots-own = У сайта есть собственная страница robots.txt: публикуется она.
seo-pages-help = Заголовок, описание, изображение для соцсетей и видимость каждой страницы: настройки страницы (панель «Страницы»).
lang-fr = Французский
lang-en = Английский
lang-de = Немецкий
lang-es = Испанский
lang-it = Итальянский
lang-pt = Португальский
lang-nl = Нидерландский

# Localization
locales-add = Добавить язык
locales-add-help = Новый язык: адреса вида /код/…, тексты переводятся здесь, в редакторе и в CMS
locales-other = Другой язык…
locales-code-input = Код языка (en, de, pt-BR…)
locales-lang = Язык
locales-code = Код
locales-prefix = Адрес
locales-primary = Основной язык
locales-progress = Перевод
locales-progress-help = Переведено текстов: { $done } из { $total }
locales-published = Опубликован
locales-published-help = Публикуется вместе с сайтом; иначе находится в подготовке (виден только в редакторе и предпросмотре)
locales-rename = Переименовать
locales-name = Отображаемое название (переключатель языков)
locales-remove = Удалить язык
locales-remove-title = Удалить «{ $name }»?
locales-remove-body = Тексты страниц, переведённые на этот язык, будут удалены (Ctrl+Z отменяет). Его файлы данных и записи останутся в папке сайта.
locales-no-url = Не указан адрес сайта (Настройки › Общие): ссылки между языками (hreflang) на страницы не добавляются.
locales-part-pages = Страницы
locales-part-pages-help = Тексты, написанные на страницах, в макетах и компонентах
locales-part-data = Данные
locales-part-data-help = Тексты файлов данных, записанные в папке языка
locales-part-entries = Содержимое
locales-part-entries-help = Записи коллекций, переводятся в CMS
locales-search = Поиск текстов
locales-todo = Нужен перевод
locales-todo-help = Только тексты, которые нужно перевести
locales-keep-source = Оставить исходный текст (перевод не нужен)
locales-translation = Перевод
locales-none = Нет текстов
locales-place-layout = Макет { $name }
locales-entry-done = Переведено
locales-auto = Перевести автоматически
locales-auto-help = Перевести ещё не переведённые тексты на этом компьютере, без подключения к интернету
locales-auto-none = Нет модели для { $from } → { $to }: скачайте язык в разделе «Настройки › Автоматический перевод»
locales-auto-nothing = Всё уже переведено
locales-auto-title = Автоматический перевод
locales-auto-texts = { $n ->
    [one] { $n } текст
    [few] { $n } текста
    [many] { $n } текстов
   *[other] { $n } текста
}
locales-auto-entries = { $n ->
    [one] { $n } запись
    [few] { $n } записи
    [many] { $n } записей
   *[other] { $n } записи
}
locales-auto-keep = Слова, которые нужно оставить как есть
locales-auto-keep-help = Названия брендов или продуктов через запятую. Название сайта сохраняется всегда.
locales-auto-start = Перевести
locales-auto-running = Перевод { $from } → { $to }
locales-auto-eta-min = около { $n } мин
locales-auto-eta-less = меньше минуты
locales-auto-eta-wait = оценка времени…
locales-auto-stop = Остановить
locales-auto-stop-help = Остановить после текущей порции (переведённое сохраняется)
locales-auto-stopping = Остановка…
locales-review = Нужна проверка
locales-review-help = Машинный перевод, который нужно проверить
locales-review-ok = Отметить как проверенный
locales-review-only = Нужна проверка
locales-review-only-help = Только машинные переводы, которые нужно проверить
locales-review-all = Отметить все как проверенные
locales-progress-auto-help = Переведено текстов: { $done } из { $total }, на проверку: { $auto }
action-add = Добавить
action-rename = Переименовать
action-move-up = Переместить выше
action-move-down = Переместить ниже
colors-opacity = Непрозрачность
colors-pick = Выбрать цвет
seo-org-logo-help = Логотип организации, который показывают поисковые системы (желательно квадратный)
seo-org-social-add = Добавьте ссылку (LinkedIn, Instagram…) и нажмите Enter
action-remove = Удалить
rows-item = Значение { $n }
rows-add = Добавить значение (Enter)
editor-locale = Отображаемый язык
editor-locale-help = Язык, показываемый на холсте
editor-locale-other-help = Изменяемые тексты записываются на этом языке; структура и стили общие для всех языков
cms-locale-help = Язык отображаемых записей
cms-locale-shared = Поля и записи общие для всех языков: изменяйте их на основном языке
cms-untranslated = Нужен перевод
cms-untranslated-help = Ещё не переведено: содержимое на основном языке; при изменении создаётся перевод
cms-auto = Машинный перевод

# CMS view
cms-collections = Коллекции
cms-search = Поиск содержимого
cms-fields-edit = Поля
cms-file = Файл
cms-no-entries = Содержимого нет.
cms-draft = Черновик
cms-open-page = Открыть страницу в редакторе
cms-shown = { $shown } из { $total }
cms-entry = Содержимое
cms-body = Текст
cms-items = { $count ->
    [one] { $count } запись
    [few] { $count } записи
    [many] { $count } записей
   *[other] { $count } записи
}
cms-fields = { $count ->
    [one] { $count } поле
    [few] { $count } поля
    [many] { $count } полей
   *[other] { $count } поля
}

# Add panel (groups, tooltip help, planned items)
insert-group-structure = Структура
insert-group-text = Текст
insert-group-media = Медиа
insert-group-cms = CMS
insert-group-forms = Формы
insert-group-code = Код
insert-help-section = Крупная часть страницы (баннер, введение…). Перетащите на страницу, чтобы разместить, или дважды щёлкните, чтобы добавить внутрь выделенного элемента.
insert-help-container = Блок, группирующий и выравнивающий другие элементы. Перетащите на страницу или дважды щёлкните.
insert-help-heading = Заголовок раздела (h2). Перетащите на страницу или дважды щёлкните.
insert-help-paragraph = Блок текста. Перетащите на страницу или дважды щёлкните.
insert-help-link = Кликабельный текст, ведущий на страницу или по адресу. Перетащите на страницу или дважды щёлкните.
insert-help-button = Ссылка в виде кнопки. Перетащите на страницу или дважды щёлкните.
insert-help-image = Оптимизированное изображение (AVIF, WebP) нескольких размеров. Перетащите на страницу или дважды щёлкните.
insert-help-list = Маркированный список. Перетащите на страницу или дважды щёлкните.
insert-help-grid = Сетка из двух колонок: в каждой ячейке могут быть элементы. Перетащите на страницу или дважды щёлкните.
insert-help-columns = Колонки рядом, которые выстраиваются друг под другом, когда не хватает места. Перетащите на страницу или дважды щёлкните.
insert-help-rich-text = Заголовок, абзац и список, которые редактируются прямо на месте. Перетащите на страницу или дважды щёлкните.
insert-help-video = Видео сайта: файл выбирается в его настройках. Перетащите на страницу или дважды щёлкните.
insert-help-icon = Линейный значок цвета текста: рисунок выбирается в его настройках. Перетащите на страницу или дважды щёлкните.
insert-help-embed = Код HTML, вставляемый на страницу как есть (встраивание сервиса, карта…). Перетащите на страницу или дважды щёлкните.
insert-help-collection-list = Один элемент на каждую запись коллекции, с фильтром, сортировкой и количеством. Перетащите на страницу или дважды щёлкните.
insert-text-rich = Заголовок
    Абзац текста.
insert-text-embed = Код HTML
insert-text-collection = Представление
block-icon = Значок
block-embed = Код HTML
block-embed-help = Вставляется на страницу как есть. Скрипты выполняются только в предпросмотре и на опубликованном сайте.
block-video-src = Источник
block-video-src-help = Видеофайл сайта (MP4, WebM) или адрес видео
block-video-poster = Постер
block-video-poster-help = Изображение, показываемое до начала воспроизведения
block-video-muted = Без звука
block-video-preload = Загрузка
block-video-preload-help = Что браузер загружает до начала воспроизведения
block-video-preload-default = На усмотрение браузера
block-video-preload-none = Ничего (при запуске)
block-video-preload-metadata = Длительность и первый кадр
block-video-preload-auto = Всё видео
picker-choose-video = Выбрать видео из медиа
field-group = Поле формы
field-group-help = Что вводит посетитель и под каким именем отправляется ответ
field-type = Тип
field-type-text = Текст
field-type-email = Эл. почта
field-type-tel = Телефон
field-type-url = Веб-адрес
field-type-number = Число
field-type-date = Дата
field-type-checkbox = Флажок
field-type-radio = Один вариант
field-type-file = Вложение
field-type-password = Пароль
field-type-hidden = Скрытое
field-name = Отправляемое имя
field-name-help = Имя ответа в полученном сообщении (буквы, цифры, дефисы)
field-required = Обязательное
field-required-help = Пока это поле пусто, форма не отправляется
field-placeholder = Текст-подсказка
field-placeholder-help = Серый текст, показываемый в пустом поле
field-rows = Видимых строк
field-auto = Автозаполнение
field-auto-help = Что браузер может предложить подставить
field-auto-default = На усмотрение браузера
field-auto-off = Выключено
field-auto-name = Полное имя
field-auto-given-name = Имя
field-auto-family-name = Фамилия
field-auto-email = Эл. почта
field-auto-tel = Телефон
field-auto-organization = Компания
field-auto-street = Адрес
field-auto-postal-code = Почтовый индекс
field-auto-city = Город
field-auto-country = Страна
block-video = Воспроизведение
block-video-controls = Элементы управления воспроизведением
block-video-autoplay = Автозапуск (без звука)
block-video-loop = Повтор
block-video-player = Видео
block-video-player-url = Адрес
block-video-player-help = Ссылка на видео YouTube или Vimeo либо видеофайл сайта; при нажатии заменяет обложку
block-video-player-file = Файл сайта
block-list = Список коллекции
block-list-computed = Список вычисляется выражением: его можно изменить в поле «Список» выше.
block-list-collection = Коллекция
block-list-also-add = + Ещё одна коллекция
block-list-also-remove = Убрать эту коллекцию из списка
block-list-also-help = Записи других коллекций, идущие после неё, в порядке коллекций (например, проекты и новости вместе).
block-list-filter = Фильтр
block-list-items = Записи
block-list-items-all = Все
block-list-items-pick = Выбранные
block-list-first = Избранные записи
block-list-first-help = Сначала эти записи в заданном порядке, затем остальные; при ограничении количества остальные дополняют список (подборка проекта, дополненная последними)
block-list-first-none = Нет
block-list-first-pick = Выбранные записи
block-list-first-field = Записи из поля «{ $field }»
block-list-first-full = Только если они заполняют список
block-list-first-full-help = При ограничении количества: если выбранных записей меньше этого числа, показывается обычный список.
block-list-contains = Только если поле
block-list-contains-help = Записи, у которых поле-список содержит текущую страницу или значение
block-list-contains-none = —
block-list-value-yes = Да
block-list-value-no = Нет
block-list-contains-value-help = Записи, поле которых содержит это значение; если пусто: значение отображаемой страницы
block-list-contains-value = содержит
block-list-contains-current = Текущая страница
block-list-all = Все записи
block-list-filter-value = Значение поля
block-list-filter-any = (заполнено)
block-list-sort = Сортировка
block-list-sort-default = Порядок коллекции
block-list-order = Порядок
block-list-asc = По возрастанию
block-list-desc = По убыванию
block-list-limit = Количество записей
block-list-limit-all = Все
block-list-more = Количество до «Показать ещё»
block-list-more-help = Сколько записей показывается сначала; следующие появляются таким же количеством при достижении конца списка (если пусто: все сразу)
block-list-date = Дата
block-bound = Связано с записью
block-bound-text = Текст
block-bound-text-label = Текст
block-bound-image = Изображение
block-bound-link = Ссылка
block-bound-written = Написанный текст
block-bound-none = Нет
block-bound-page = Страница записи
insert-group-blocks = Блоки
insert-block-hero = Шапка страницы
insert-block-features = Преимущества
insert-block-text-image = Текст и изображение
insert-block-cta = Призыв к действию
insert-block-testimonial = Отзыв
insert-block-faq = Вопросы
insert-block-contact = Контакты
insert-help-block = Готовая секция для редактирования (тексты, настраиваемые стили). Перетащите на страницу или дважды щёлкните.
insert-cart = Корзина и заказ
insert-help-cart = Корзина сайта и форма заказа, отправляемого по электронной почте (форма сайта, без онлайн-оплаты). Заполняется кнопками «В корзину».
insert-cart-add = В корзину
insert-help-cart-add = Кнопка, добавляющая товар в корзину сайта: товар и цена вводятся вручную или берутся из полей записи.
insert-text-cart-add = В корзину
insert-grid = Сетка
insert-columns = Колонки
insert-rich-text = Форматированный текст
insert-video = Видео
insert-icon = Значок
insert-lottie = Lottie
insert-collection-list = Список коллекции
insert-locale-switch = Переключатель языков
insert-help-locale-switch = По одной ссылке на каждый язык сайта, ведущей на ту же страницу (текущий язык выделен)
insert-text-locale-switch = Языки
insert-form = Форма
insert-embed = Код HTML

# Canvas: selection navigation, moves, view
editor-select-child = Выбрать первый дочерний элемент
editor-select-parent = Выбрать родителя
editor-select-sibling = Следующий, предыдущий элемент
editor-move = Переместить элемент
editor-move-copy = Копировать при перемещении
editor-order = Переместить выше, ниже
editor-zoom-selection = Масштаб по выделению
editor-pan = Сдвинуть вид
key-child = Enter
key-parent = Shift+Enter
key-siblings = Tab · Shift+Tab
key-move-drag = Перетаскивание
key-copy-drag = Alt+перетаскивание
key-order = Alt+↑ · Alt+↓
key-wrap = Ctrl+G
key-zoom-sel = Shift+2
key-pan = Пробел+перетаскивание
editor-copy = Копировать
editor-cut = Вырезать
editor-paste = Вставить
editor-unwrap = Убрать обёртку
key-copy = Ctrl+C
key-cut = Ctrl+X
key-paste = Ctrl+V
key-unwrap = Ctrl+Shift+G
cms-entry-title = Заголовок
action-create = Создать
cms-delete-title = Удалить «{ $name }»?
cms-delete-body = Файл содержимого будет удалён. Ctrl+Z вернёт его, пока сайт открыт.
cms-gallery-add = Добавить
cms-gallery-add-help = Выберите одно или несколько изображений из медиа (или перетащите файлы сюда)
cms-gallery-replace = Щёлкните, чтобы заменить
cms-gallery-remove = Убрать из списка
cms-gallery-item = Изображение { $n }: { $name }
cms-gallery-count = { $count ->
    [0] Нет изображений
    [one] { $count } изображение
    [few] { $count } изображения
    [many] { $count } изображений
   *[other] { $count } изображения
}
cms-gallery-count-files = { $count ->
    [0] Нет файлов
    [one] { $count } файл
    [few] { $count } файла
    [many] { $count } файлов
   *[other] { $count } файла
}
cms-gallery-hint = перетащите, чтобы изменить порядок
cms-panel-resize = Перетащите, чтобы изменить размер панели (двойной щелчок: ширина по умолчанию)
cms-template = Шаблон страницы
cms-template-edit = Изменить шаблон страницы
cms-template-help = Макет, общий для всех страниц коллекции: его изменение обновляет все эти страницы, кроме тех, у которых собственный дизайн
cms-template-page = Шаблон страниц · { $name }
cms-pin-on = Показывать это поле столбцом таблицы
cms-pin-off = Убрать это поле из столбцов таблицы
cms-columns = Столбцы таблицы
cms-column-hide = Скрыть столбец «{ $name }»
cms-column-left = Переместить влево
cms-column-right = Переместить вправо
cms-column-drag = Перетащите, чтобы переместить столбец
cms-column-widths-reset = Автоматическая ширина
cms-column-resize = Перетащите, чтобы задать ширину (двойной щелчок: автоматически)
cms-media-path = Адрес (или выберите из медиа)
cms-tag-duplicate = Это значение уже есть в списке
cms-tag-help = Перетащите, чтобы изменить порядок (Alt+стрелки); двойной щелчок: изменить
cms-ref-order-help = Перетащите, чтобы изменить порядок отображения (Alt+стрелки)
cms-ref-full = Достигнут максимум ({ $max }): удалите запись, чтобы добавить другую
cms-repeat-fold-all = Свернуть все
cms-repeat-unfold-all = Развернуть все
cms-repeat-drag = Перетащите, чтобы изменить порядок
cms-repeat-duplicate = Дублировать
cms-pattern-none = Произвольный текст
cms-pattern-slug = Адрес (строчные буквы и дефисы)
cms-pattern-email = Адрес эл. почты
cms-pattern-url = Ссылка (https://…)
cms-pattern-phone = Номер телефона
cms-pattern-postcode = Почтовый индекс (5 цифр)
cms-pattern-custom = Свой (выражение)
cms-tag-add = Добавить…

# Rich text (step 3.2)
rt-toolbar = Форматирование
rt-block = Тип абзаца
rt-p = Абзац
rt-h2 = Заголовок 2
rt-h3 = Заголовок 3
rt-h4 = Заголовок 4
rt-quote = Цитата
rt-bold = Жирный (Ctrl+B)
rt-italic = Курсив (Ctrl+I)
rt-link = Ссылка (Ctrl+K); пустой адрес удаляет ссылку
rt-link-apply = Применить
rt-ul = Маркированный список
rt-ol = Нумерованный список
rt-clear = Очистить форматирование
rt-source-md = Показать и изменить Markdown
rt-source-html = Показать и изменить HTML
cms-body-empty = Напишите текст содержимого…
export-title = Экспорт сайта
export-lead = Файлы сайта (страницы, стили, оптимизированные изображения), скопированные в папку, чтобы разместить сайт в другом месте или заархивировать. Для предпросмотра и публикации это не нужно.
export-env-help = Переменные выбранного окружения (адрес сайта, настройки тестового окружения…)
export-images-help = Варианты AVIF и WebP нескольких размеров, как на опубликованном сайте; в первый раз дольше
export-where = Папка
export-run = Экспортировать
export-running = Экспорт…
export-reveal = Показать папку
export-failed = Не удалось экспортировать. { $message }
publish-sync-versions = Синхронизировать версии перед публикацией
publish-sync-versions-help = Сохраняет изменения, получает сделанные в других местах (Pages CMS) и отправляет их в репозиторий сайта (Настройки › Версии)
publish-syncing = Синхронизация версий…
publish-sync-message = Публикация
publish-building = Подготовка файлов сайта…

# Collection schema (step 3.3)
cms-required = Обязательное поле
cms-required-mark = Обязательное
cms-pattern = Недопустимый формат
cms-sub-default = Текст
cms-sub-new = Новое поле группы
cms-sub-add = Добавить поле
cms-sub-remove = Удалить поле (значения останутся в записях)
cms-sub-locked = Определено компонентом .pages.yml: изменить можно в этом файле.
cms-block-new = Новый тип блока
cms-block-add = Добавить тип блока
cms-block-remove = Удалить этот тип блока (уже написанные блоки останутся в записях)
cms-repeat-up = Переместить выше
cms-repeat-down = Переместить ниже
cms-repeat-remove = Удалить
cms-repeat-add = Добавить
cms-repeat-add-block = Добавить блок
cms-json = Недопустимое значение: { $detail }
cms-more = Другие значения ({ $count })
cms-more-help = Скрытые поля и значения, отсутствующие в схеме (.pages.yml)
cms-undeclared = Нет в .pages.yml: Pages CMS удалит это при сохранении содержимого.
cms-undeclared-add = Добавить в схему
cms-undeclared-name = Имя отклонено Pages CMS (допустимы только буквы, цифры, - и _)
cms-fields-help = Поля коллекции (схема, общая с Pages CMS)
cms-fields-count = { $count ->
    [one] { $count } поле
    [few] { $count } поля
    [many] { $count } полей
   *[other] { $count } поля
}
cms-schema-inferred = Поля определены по содержимому: первое изменение опишет их в .pages.yml (Pages CMS).
cms-field-new = Новое поле
cms-field-new-title = Новое поле: { $type }
cms-field-label = Подпись
cms-field-name = Имя в файлах
cms-field-name-help = Ключ, записываемый в каждый файл содержимого; после создания поля он не меняется.
cms-field-type = Тип
cms-field-component = Определено компонентом «{ $name }» в .pages.yml, общим с другими полями.
cms-field-help = Подсказка
cms-field-help-placeholder = Текст, показываемый при наведении на подпись
cms-field-required = Обязательное
cms-field-list = Несколько значений
cms-field-hidden = Скрыто в форме
cms-field-hidden-help = Остаётся в содержимом, но не предлагается для ввода ни здесь, ни в Pages CMS.
cms-field-column = Столбец таблицы
cms-field-column-chip = Столбец
cms-field-primary = Заголовок записи
cms-field-primary-help = Название записи в таблице и в верхней части её формы.
cms-field-primary-chip = Заголовок
cms-field-drag = Перетащите, чтобы изменить порядок
cms-field-up = Переместить выше
cms-field-down = Переместить ниже
cms-field-remove = Удалить
cms-field-remove-title = Удалить поле «{ $name }»?
cms-field-remove-body = Оно исчезнет из формы и из Pages CMS; уже введённые значения останутся в содержимом. Ctrl+Z вернёт его.
cms-field-retype-title = Изменить тип поля «{ $name }»?
cms-field-retype-body = Уже введённые значения не преобразуются.
cms-field-retype = Изменить
cms-field-time = С временем
cms-field-format = Формат
cms-field-choices = Варианты
cms-field-choice-add = Добавить вариант…
cms-field-multiple = Несколько вариантов
cms-field-collection = Коллекция
cms-field-ref-label = Отображаемая подпись
cms-field-pattern = Шаблон
cms-field-pattern-help = Регулярное выражение, которому должно соответствовать значение, например ^[a-z0-9-]+$
cms-field-pattern-invalid = Недопустимое выражение
cms-field-subfields = Поля группы
cms-field-blocks = Типы блоков
type-string = Текст
type-text = Длинный текст
type-rich-text = Форматированный текст
type-image = Изображение
type-file = Файл
type-date = Дата
type-boolean = Да / нет
type-number = Число
type-select = Выбор
type-reference = Ссылка на запись
type-code = Код
type-object = Группа полей
type-block = Блоки
type-uuid = Идентификатор
cms-collection-name = Название коллекции
cms-new-title = Заголовок
cms-new-slug = Адрес страницы
cms-new-slug-help = Часть адреса, уникальная для каждой записи; не меняйте её после публикации (SEO).
editor-view = Вид
editor-rulers = Линейки
key-rulers = Ctrl+R
editor-guides = Направляющие колонок
editor-guides-count = Колонки
editor-guides-gutter = Желоб (px)
editor-guides-margin = Поля (px)
editor-guides-max = Макс. ширина (px)
editor-guides-max-help = Максимальная ширина содержимого; если пусто: на всю ширину
key-up = Alt+↑
key-down = Alt+↓
dock-dock = Закрепить
dock-close-short = Закрыть
cms-entry-open = Открыть
editor-bp-kind-max-help = Стили для экранов уже этой ширины
editor-bp-kind-min-help = Стили для экранов шире этой ширины
editor-bp-width-help = Ширина (px): стили этого брейкпоинта действуют по ней. Она остаётся между соседними брейкпоинтами.
preview-options = Параметры предпросмотра
preview-here = В рабочей области
preview-window = В новом окне
key-preview-window = Shift+щелчок
preview-tablet = Планшет
preview-mobile = Телефон
preview-reload = Обновить
preview-close = Закрыть
preview-close-help = Вернуться в текущее пространство (Esc)
inspector-locale-help = Перевод: исходный текст показан серым
pages-seo-locale = Переведённые заголовок и описание; адрес, изображение и индексация общие для всех языков
pages-seo-locale-data = SEO берётся из файла данных: переведите его в разделе «Локализация › Данные»
cms-translated-help = Переведено на этот язык

# Réglages lisibles (3.12)
seo-readable = Заголовок и публикации в соцсетях
seo-readable-help = Настройки, извлечённые из шаблонов сайта: каждый выбор переписывает соответствующее вычисление (можно отменить)
seo-tab-title = Заголовок вкладки
seo-tab-title-help = Заголовок, показываемый на вкладке браузера и в результатах поиска; у страницы без заголовка показывается название сайта
seo-title-first = Страница | Сайт
seo-site-first = Сайт | Страница
seo-title-only = Только страница
seo-title-sep = Разделитель
seo-title-sample = Заголовок страницы
seo-title-sample-help = Пример заголовка вкладки
seo-share-default = Изображение для соцсетей по умолчанию
seo-canonical-base = Официальный адрес
seo-canonical-base-help = Адрес, с которого начинаются канонические ссылки и изображения страниц для соцсетей (https://…)
seo-canonical-invalid = Недопустимый адрес (ожидается: https://…)
settings-menus = Списки ссылок шаблонов
settings-navigation = Навигация
menus-help = Списки ссылок из шаблонов (меню сайта): текст, адрес и порядок каждой ссылки
menus-none = В шаблонах сайта не найдено списков ссылок
menus-label = Текст
menus-url = Адрес
menus-new = Новая ссылка
menus-add = Добавить ссылку
picks = Избранное
picks-help = Записи, выбранные из коллекций и показываемые страницей, по порядку (идентификаторы записаны в данных сайта)
picks-missing = Нет записи с идентификатором «{ $slug }»
picks-add = Добавить запись…

# Settings › AI assistants (4.2)
settings-mcp = ИИ-ассистенты
settings-mcp-help = Claude, ChatGPT, Codex или локальная модель читают и изменяют сайт, открытый в Revel, теми же действиями, что и вы: каждое изменение сразу видно и отменяется через Ctrl+Z.
mcp-server = Локальный сервер
mcp-server-help = Подключение для ассистентов, установленных на этом компьютере (только 127.0.0.1, нужен ключ доступа). Если выключено, ни один ассистент не сможет подключиться к Revel.
mcp-state-on = Включён · { $addr }
mcp-state-off = Выключен
mcp-state-port-busy = Порт уже занят
mcp-state-failed = Не удалось запустить
mcp-port = Порт
mcp-port-help = Локальный порт сервера (от 1024 до 65535); меняйте его, только если он уже занят другой программой
mcp-key = Ключ доступа
mcp-key-help = Нужен серверу. Ассистенты, запущенные командой Revel, читают его сами; вставлять его нужно только в Open WebUI.
mcp-key-show = Показать ключ
mcp-key-hide = Скрыть ключ
mcp-key-copy = Скопировать ключ
mcp-key-renew = Обновить
mcp-key-renew-help = Новый ключ: старый перестанет работать (замените его в Open WebUI)
mcp-key-renew-title = Обновить ключ доступа?
mcp-connect = Подключить ассистента
mcp-connect-off = Сначала включите локальный сервер
mcp-where-claude-desktop = Claude › Settings › Developer › Edit Config
mcp-where-claude-code = Терминал
mcp-where-lm-studio = LM Studio › Program › Install › Edit mcp.json
mcp-where-open-webui = Open WebUI › Admin Settings › Integrations › External Tool Servers › Add Connection
mcp-how-claude-desktop = Откройте файл из меню Claude в строке меню, вставьте конфигурацию (если в нём уже есть серверы, добавьте «revel» только внутрь «mcpServers»), сохраните, затем закройте и снова откройте Claude.
mcp-how-claude-code = Выполните команду один раз в терминале: после этого Revel будет доступен в любом проекте.
mcp-how-codex = Добавьте в конец файла (создайте его, если его нет), затем перезапустите: файл общий для приложения ChatGPT, командной строки Codex и расширения для редактора кода.
mcp-how-lm-studio = Вставьте в mcp.json (если в нём уже есть серверы, добавьте «revel» только внутрь «mcpServers») и сохраните.
mcp-how-open-webui = Open WebUI версии 0.6.31 или новее: скопируйте тип, адрес, аутентификацию и ключ в новое подключение. Если Open WebUI установлен через Docker: адрес для Docker.
mcp-openapi-help = Описание OpenAPI тех же действий для инструментов без MCP (скрипты, автоматизации, серверы инструментов OpenAPI): POST /api/<tool>, аутентификация Bearer с ключом доступа.
mcp-field-type = Тип
mcp-field-url = Адрес
mcp-field-url-docker = Адрес (Docker)
mcp-field-auth = Аутентификация
mcp-field-key = Ключ
mcp-activity = Последние действия
mcp-activity-empty = Действий пока нет
mcp-client-unknown = Ассистент
mcp-tool-set_text = Текст изменён
mcp-tool-set_attributes = Настройки элемента изменены
mcp-tool-set_styles = Стили изменены
mcp-tool-insert_html = Элементы добавлены
mcp-tool-remove_node = Элемент удалён
mcp-tool-set_data = Данные изменены
mcp-tool-set_entry_field = Содержимое CMS изменено
mcp-tool-apply_commands = Изменения
mcp-tool-undo = Отмена
mcp-tool-redo = Повтор
action-copy = Копировать

# Bouton « Publier » de la barre supérieure (5.1)
pub-panel = Публикация
pub-public = Публичный сайт
pub-test = Тестовая версия
pub-never = Ещё не публиковалось
pub-at-now = Опубликовано только что
pub-at-min = Опубликовано { $n } мин назад
pub-at-hours = Опубликовано { $n } ч назад
pub-at-date = Опубликовано { $date }
pub-changes = { $n ->
    [one] { $n } страница изменена
    [few] { $n } страницы изменены
    [many] { $n } страниц изменено
   *[other] { $n } страницы изменено
}
pub-files-changed = Стили или медиа изменены
pub-up-to-date = Всё актуально
pub-kind-added = Новая
pub-kind-changed = Изменена
pub-kind-removed = Удалена
pub-files = Стили, медиа и скрипты
pub-live = В сети
pub-waiting = Ожидание
pub-needs-access = Нужен доступ
pub-hosting = Хостинг
pub-hosting-help = Управление хостингом сайта (Настройки › Публикация)
pub-empty = Ещё не в сети
pub-setup = Запустить сайт
pub-dirty = Неопубликованные изменения
pub-page-open = Открыть в редакторе
pub-select = Включить в эту публикацию

# Guided go-live (5.2, 5.3)
setup-title = Запуск сайта
setup-host = Хостинг-провайдер
setup-host-pick = Выберите провайдера
setup-other = Другой
setup-other-help = Другой провайдер с доступом по SFTP
setup-console = Личный кабинет
setup-paste = Сообщение от хостинга
setup-paste-help = Вставьте письмо с настройками или страницу доступа: сервер, имя пользователя и пароль будут распознаны
setup-paste-placeholder = Вставьте сюда (необязательно)
setup-env-help = Публичный сайт: адрес вашего сайта. Тестовая версия: отдельный адрес, чтобы всё проверить перед публикацией публичного сайта
setup-advanced = Дополнительные настройки
setup-folder = Папка сайта
setup-connect = Подключиться
setup-connecting = Подключение к серверу…
setup-connected = Подключено
host-ovh-sftp = Включите SFTP: Web Cloud › Hosting plans › ваш тариф › FTP - SSH, столбец «SFTP»
host-ovh-access = Доступ: на той же вкладке, «FTP and SFTP server» и «Main login»
host-o2switch-ip = Разрешите этот компьютер: cPanel › Outils › «Autorisation SSH» › «Ajouter une exception», порт 22
host-o2switch-access = Доступ: имя пользователя и пароль cPanel (письмо «Bienvenue chez o2switch»)
host-infomaniak-access = Доступ: Manager › hosting & website management › ваш хостинг › FTP / SSH
host-hostinger-ssh = Включите SSH: hPanel › Websites › Dashboard › SSH Access › Enable
host-hostinger-access = Доступ: в том же разделе (IP для FTP, порт 65002, имя пользователя); пароль FTP
host-ionos-create = Создайте доступ: Menu › Hosting › SFTP & SSH › Set up
host-ionos-access = Доступ: SFTP & SSH › Manage › ваше имя пользователя

# Pre-publish checks (5.4)
check-title = Перед публикацией
check-broken-link = { $n ->
    [one] { $n } нерабочая ссылка
    [few] { $n } нерабочие ссылки
    [many] { $n } нерабочих ссылок
   *[other] { $n } нерабочей ссылки
}
check-heavy-image = { $n ->
    [one] { $n } изображение размером более 1 MB
    [few] { $n } изображения размером более 1 MB
    [many] { $n } изображений размером более 1 MB
   *[other] { $n } изображения размером более 1 MB
}
check-missing-alt = { $n ->
    [one] { $n } изображение без альтернативного текста
    [few] { $n } изображения без альтернативного текста
    [many] { $n } изображений без альтернативного текста
   *[other] { $n } изображения без альтернативного текста
}
check-no-title = { $n ->
    [one] { $n } страница без заголовка
    [few] { $n } страницы без заголовка
    [many] { $n } страниц без заголовка
   *[other] { $n } страницы без заголовка
}
publish-anyway = Всё равно опубликовать

# Back to the previous publish (5.5)
pub-rollback-help = Вернуться к версии, опубликованной { $date }
pub-rollback-title = Вернуть в сеть версию от { $date }?
pub-rollback-confirm = Вернуться
publish-restoring = Подготовка предыдущей версии…

# Domain name (5.7)
domain-title = Доменное имя
domain-name = Домен
domain-records = Записи, которые нужно создать
domain-records-help = У регистратора домена (зона DNS): записи A для домена, CNAME для www
domain-type = Тип
domain-host = Имя
domain-value = Значение
domain-ipv6 = IPv6 (необязательно)
domain-other = У регистратора домена: записи, указанные вашим хостингом
domain-check = Проверить
domain-check-help = Изменение домена может примениться в течение 48 часов
domain-checking = Проверка домена…
domain-none = { $domain } не указывает ни на какой адрес
domain-elsewhere = { $domain } указывает на { $addresses }, а не на хостинг
domain-points = { $domain } указывает на { $addresses }
domain-https = HTTPS включён
domain-no-https = HTTPS пока не включён
domain-serves = Домен показывает ваш сайт
domain-other-page = Домен показывает не ваш сайт, а другую страницу
host-ovh-domain-1 = OVHcloud Control Panel › Web Cloud › ваш тариф хостинга › добавьте домен как внешний («Mes sites» / Multisite)
host-ovh-domain-2 = У регистратора домена: записи A и TXT «ovhcontrol», которые показывает OVHcloud
host-o2switch-domain-1 = cPanel › Domaines › «Domaines configurés»: добавьте домен
host-o2switch-domain-2 = У регистратора домена: серверы имён ns1.o2switch.net и ns2.o2switch.net или запись A на общий IP-адрес (cPanel)
host-infomaniak-domain-1 = Manager › ваш сайт › Information: адреса IPv4 и IPv6
host-infomaniak-domain-2 = У регистратора домена: записи A (и AAAA) на эти адреса
host-hostinger-domain-1 = hPanel › Websites: добавьте сайт, затем «Check guide»
host-hostinger-domain-2 = У регистратора домена: показанные серверы имён или записи A для @ и www на IP-адрес; DNSSEC выключен
host-ionos-domain-1 = Menu › Domains & SSL › настройте внешний домен
host-ionos-domain-2 = У регистратора домена: серверы имён IONOS или подтверждающая запись TXT на @

# Forms (5.8)
insert-help-form = Контактная форма: сообщения приходят по электронной почте через ваш хостинг (Настройки › Формы). Перетащите на страницу или дважды щёлкните.
forms-help = Сообщения из форм сайта отправляются по электронной почте вашим хостингом (PHP), без внешнего сервиса
forms-recipient = Адрес получателя
forms-subject = Тема письма
forms-subject-default = Новое сообщение с сайта { $site }
forms-success = Сообщение после отправки
forms-success-default = Спасибо, ваше сообщение отправлено.
forms-host-yes = { $name }: формы принимаются
forms-host-no = { $name }: формы недоступны (нет PHP)
forms-infomaniak = Infomaniak: включите функцию отправки почты сайта (расширенные настройки › PHP / Apache) и укажите адрес отправителя; не более 50 сообщений за 24 ч
forms-from-domain = Сообщения отправляются с адреса site@ вашего домена: домен должен быть привязан к хостингу
cms-open-view = Открыть в CMS
new-site-background = Фон
new-site-dark = Тёмный
new-site-light = Светлый
editor-frame-width = Ширина страницы
editor-frame-width-help = Ширина страницы: { $width } px. Перетащите, чтобы изменить (экран следует за ней), двойной щелчок: ширина экрана

# Component variants (component canvas)
variant-primary = Основной
variant-hover = Наведение
variant-pressed = Нажатие
variant-label = Вариант
variant-new = Новый вариант
variant-name = Название варианта
variant-default-name = Вариант { $n }
variant-create = Создать
variant-rename = Переименовать вариант
variant-remove = Удалить вариант
variant-remove-title = Удалить вариант «{ $name }»?
variant-add = Добавить вариант
variant-add-hover = Наведение
variant-add-pressed = Нажатие
variant-add-custom = Свой вариант…
variant-add-click = По щелчку…
variant-on-click = Включается по щелчку на компоненте
variant-arrow-hover = При наведении
variant-arrow-pressed = Пока нажат
variant-arrow-click = По щелчку
variant-arrow-click-help = Щелчок по компоненту включает этот вариант, следующий возвращает основной
variant-transition = Переход
variant-transition-help = Анимированная смена на варианты наведения и нажатия
variant-transition-none = Без перехода
variant-transition-fast = Быстрый · 0,15 с
variant-transition-smooth = Плавный · 0,3 с
variant-transition-slow = Медленный · 0,6 с
variant-scope = Стили варианта «{ $name }»
component-back-help = Вернуться к странице
component-of = Используемый компонент
component-bar = Открыть компонент

# Text format bar (canvas)
format-bold = Жирный (Ctrl+B)
format-italic = Курсив (Ctrl+I)
format-link = Ссылка (пустой адрес: ссылка удаляется)
format-span = Фрагмент: отдельно оформленный текст (затем выберите его в слоях)
format-url = Адрес ссылки, Enter для подтверждения

# Settings added after the site recreation (D-73)
unit-deg = градусы
unit-turn = обороты
unit-ms = миллисекунды
unit-s = секунды
unit-percent-self = проценты от самого элемента
unit-fr = доля оставшегося места
unit-min-content = ширина самого длинного слова
unit-max-content = ширина содержимого в одну строку
unit-min-content-s = Мин.
unit-max-content-s = Макс.
fx-blur = Размытие
fx-brightness = Яркость
fx-contrast = Контрастность
fx-grayscale = Оттенки серого
fx-saturate = Насыщенность
fx-sepia = Сепия
fx-invert = Инверсия
fx-hue-rotate = Оттенок
fx-move-x = Сдвиг по X
fx-move-y = Сдвиг по Y
fx-rotate = Поворот
fx-scale = Масштаб
fx-scale-x = Масштаб по X
fx-scale-y = Масштаб по Y
fx-skew-x = Наклон по X
fx-skew-y = Наклон по Y
fx-inherited = Унаследовано
fx-remove = Удалить
fx-transform-add = Добавить трансформацию
fx-transform-none = Без трансформаций
fx-filter-add = Добавить фильтр
fx-filter-none = Без фильтров
fx-backdrop-add = Добавить фильтр фона
tr-all = Всё
tr-opacity = Непрозрачность
tr-transform = Трансформация
tr-color = Цвет текста
tr-background = Цвет фона
tr-border = Цвет границы
tr-shadow = Тень
tr-filter = Фильтры
tr-width = Ширина
tr-height = Высота
ease-ease = Плавно
ease-in = С ускорением
ease-out = С замедлением
ease-in-out = Ускорение, затем замедление
ease-linear = Постоянная скорость
ease-expo-out = Сильное замедление
tr-none = Без перехода
tr-add = Добавить переход
tr-remove = Удалить переход
tr-property = Что меняется
tr-duration = Длительность
tr-delay = Задержка
tr-easing = Кривая
grid-equal = Равные
grid-equal-help = Колонки одинаковой ширины
grid-custom = Свои
grid-custom-help = Ширина каждой колонки (fr: доля оставшегося места)
grid-auto = Авто
grid-auto-help = Столько колонок, сколько поместится, с минимальной шириной
grid-track = Ширина колонки
grid-track-add = Добавить колонку
grid-auto-min = Минимальная ширина колонки
grid-auto-fit = Растянуть
grid-auto-fit-help = Колонки растягиваются, когда элементов меньше, чем мест
styles-row-gap = Промежуток между строками
styles-column-gap = Промежуток между колонками
gap-link = Одинаковый промежуток для строк и колонок
self-auto = Как у остальных
styles-grid-span = Ячейки
grid-span-columns = Занято колонок
grid-span-rows = Занято строк
styles-self = Выравнивание
styles-order = Порядок
ws-normal = Автоматически
ws-nowrap = Никогда (одна строка)
ws-pre-line = Сохранять введённые переносы строк
ws-pre-wrap = Сохранять пробелы и переносы строк
styles-white-space = Перенос строк
tw-wrap = Обычный
tw-balance = Сбалансированные строки
tw-pretty = Без одиноких слов в конце
styles-text-wrap = Разбиение
styles-blend = Наложение
styles-transform = Трансформации
styles-transform-origin = Начало отсчёта
styles-clip-path = Обрезка
styles-text-shadow = Тень текста
clip-none = Нет
clip-circle = Круг
clip-ellipse = Эллипс
clip-inset = Скруглённый прямоугольник
clip-triangle = Треугольник
clip-rhombus = Ромб
clip-parallelogram = Параллелограмм
clip-arrow = Стрелка
clip-slant = Скошенный низ
clip-custom = Свой многоугольник
clip-x = Положение точки по горизонтали
clip-y = Положение точки по вертикали
clip-remove = Удалить эту точку
clip-add = Добавить точку
styles-transition = Переходы
styles-filter = Фильтры
styles-backdrop = Фильтры фона
blend-normal = Обычный
blend-multiply = Умножение
blend-screen = Экран
blend-overlay = Перекрытие
blend-darken = Замена тёмным
blend-lighten = Замена светлым
blend-color-dodge = Осветление основы
blend-color-burn = Затемнение основы
blend-hard-light = Жёсткий свет
blend-soft-light = Мягкий свет
blend-difference = Разница
blend-exclusion = Исключение
blend-hue = Тон
blend-saturation = Насыщенность
blend-color = Цветность
blend-luminosity = Яркость

block-bound-bg = Фоновое изображение
block-bound-date = Формат даты
block-bound-class = Добавляемый класс
block-bound-class-help = Класс, добавляемый элементу, когда поле заполнено (со своим стилем)
block-bound-class-name = Класс
block-bound-fallback = Иначе
block-bound-fallback-help = Поле, показываемое, когда первое пусто
block-bound-date-raw = Как сохранено
block-bound-before = Текст перед
block-bound-after = Текст после
block-bound-count = Количество «{ $field }»
block-bound-value-class = Класс из значения
block-bound-value-class-help = Класс, составленный из заданного начала и значения поля (block--full, block--column): для каждого значения в стилях нужно одно правило.
block-bound-value-class-none = Нет
block-bound-value-class-prefix = Начало класса
block-bound-value-class-default = Если поле пусто
block-bound-value-class-add = Класс из значения поля
block-bound-emphasis = Выделять *слова*
block-bound-emphasis-help = Слова, написанные в поле между звёздочками («наша *экспертиза*»), получают класс (цвет, насыщенность), заданный в стилях.
block-bound-emphasis-class = Класс выделенных слов
block-bound-visibility = Отображение
cond-set = задано
cond-empty = пусто
cond-equals = равно
cond-differs = отличается от
cond-contains = содержит
cond-more = содержит больше (значений)
cond-less = содержит меньше (значений)
cond-field = Проверяемое поле
cond-test = Проверка
cond-value = Значение
cond-count = Количество значений
cond-remove = Удалить это условие
cond-add = Добавить условие
cond-mode = Выполняемые условия
cond-mode-all = Все эти условия
cond-mode-any = Хотя бы одно из этих условий
cond-class-add = Класс по условию
cond-class-remove = Удалить этот класс
cond-always = Показывается всегда
cond-raw-help = Условие записано кодом: замените его настраиваемыми условиями
cond-replace = Заменить настраиваемым условием
block-bound-always = Всегда
block-bound-if-set = Если «{ $field }» задано
block-bound-if-empty = Если «{ $field }» пусто
block-list-exclude = Без показанной записи
block-list-exclude-help = На странице записи список её не показывает («другие проекты»); запись, поставленная первой, остаётся в списке, даже если это отображаемая запись.
cms-collection-folder = Папка содержимого
cms-collection-folder-new = Новая папка
cms-collection-folder-existing = { $folder } (записей: { $count })
block-list-source = Источник
block-list-of-field = «{ $field }» этой записи
block-bound-value = Значение
insert-help-lottie = Анимация, экспортированная из After Effects (Bodymovin) или LottieFiles; воспроизводится самим сайтом, без внешнего сервиса
block-lottie = Анимация Lottie
block-lottie-file = Файл
block-lottie-none = Нет файла
block-lottie-choose = Выбрать анимацию
block-lottie-play = Воспроизведение
lottie-play-auto = На экране
lottie-play-auto-help = Воспроизводится, пока видна, иначе приостановлена
lottie-play-hover = При наведении
lottie-play-hover-help = При наведении воспроизводится вперёд, затем назад
lottie-play-click = При нажатии
lottie-play-click-help = Проигрывается заново при каждом нажатии
lottie-play-scroll = При прокрутке
lottie-play-scroll-help = Следует за прокруткой страницы
block-lottie-speed = Скорость
block-lottie-loop = Повтор
act-title = Действия
act-help = Нажатие, наведение или прокрутка действуют на этот или другой элемент (меню, шапку, баннер)
act-add = Добавить действие
act-remove = Удалить действие
act-trigger = Запуск
act-trigger-click = При нажатии
act-trigger-hover = При наведении
act-trigger-scroll = При прокрутке
act-trigger-view = На экране
act-trigger-load = При загрузке
act-scroll-px = После (прокручено пикселей)
act-action = Действие
act-class = Добавить класс
act-class-help = Класс добавляется и убирается при следующем нажатии (или когда наведение заканчивается, при обратной прокрутке)
act-toggle = Показать или скрыть
act-toggle-help = Показывает, если скрыт, и скрывает, если виден
act-show = Показать
act-show-help = Цель появляется
act-hide = Скрыть
act-hide-help = Цель исчезает
act-play = Воспроизвести или приостановить
act-play-help = Видео цели воспроизводится или приостанавливается
act-dismiss = Закрыть навсегда
act-dismiss-help = Скрывается и больше не показывается посетителю (баннер согласия)
act-random = Случайный дочерний элемент
act-random-help = Показывает один случайный дочерний элемент цели (девиз, цитата), остальные скрыты
act-random-delay = Задержка (мс)
act-spin = Вращение
act-spin-help = Каждый раз добавляет цели поворот плюс случайную четверть оборота (компас, колесо)
act-spin-deg = Угол (градусы)
act-sound = Воспроизвести звук
act-sound-help = Один раз воспроизводит звук (случайный, если их несколько); отключается переключателем «Звуки»
act-music = Запустить музыку
act-music-help = Воспроизводит музыку по кругу, случайно выбранную из отмеченных; заменяет предыдущую; отключается переключателем «Музыка»
act-sound-add = Добавить звуковой файл
act-sound-remove = Удалить этот файл
act-sound-volume = Громкость (%)
act-class-invalid = Имя класса: только буквы, цифры, дефисы и подчёркивания
act-class-name = Класс
act-target = Цель
act-target-self = Этот элемент
act-target-parent = Родительский элемент
act-target-next = Следующий элемент
act-target-body = Вся страница
act-target-id = Элемент «{ $id }»
act-target-class = Класс «{ $class }»
act-target-closest = Ближайший «{ $class }» выше по структуре
insert-tabs = Вкладки
insert-help-tabs = Одна кнопка на каждую панель; выбранная панель показывается на сайте
insert-text-tabs = Вкладка
insert-group-interactive = Интерактив
insert-navbar = Панель навигации
insert-help-navbar = Бренд и ссылки; на планшете и телефоне ссылки открываются кнопкой «Меню»
insert-text-navbar = Бренд|Меню|Главная|О нас|Контакты
insert-ticker = Бегущая строка
insert-help-ticker = Её элементы бесконечно прокручиваются (логотипы, команда, слова); скорость и направление задаются вверху панели «Стили»
insert-text-ticker = Элемент
insert-carousel = Карусель
insert-help-carousel = Слайды рядом, стрелки и счётчик; листается свайпом или прокруткой
insert-slideshow = Слайд-шоу
insert-help-slideshow = Один слайд за раз, автозапуск, точки
insert-text-slider = Слайд|Назад|Вперёд
insert-consent = Согласие на cookie
insert-help-consent = Баннер показывается, пока посетитель не сделает выбор; Google Analytics загружается после согласия (идентификатор вверху панели «Стили»)
insert-text-consent = С вашего согласия мы измеряем посещаемость сайта.|Отказаться|Принять
insert-switch = Переключатель звука
insert-help-switch = Кнопка «Вкл / Выкл», отключающая музыку или звуки сайта (настройка вверху панели «Стили»)
insert-text-switch = Музыка|Вкл|Выкл
insert-filters = Фильтры списка
insert-help-filters = Кнопки, фильтрующие список на странице (проекты по направлению); отметьте его элементы как «Фильтруемый элемент» вверху панели «Стили»
insert-text-filters = Все|Вариант 1|Вариант 2
insert-video-player = Видеоплеер
insert-help-video-player = Видео YouTube, Vimeo или видео сайта, запускаемое нажатием на обложку; адрес задаётся вверху панели «Стили»
insert-text-video-player = Смотреть видео
ticker-title = Бегущая строка
items-title = Элементы
items-replace = Заменить изображение
items-up = Переместить выше
items-down = Переместить ниже
items-add-image = Добавить изображение
items-item = Элемент { $n }
items-tabs = Вкладки
items-tab = Вкладка { $n }
items-add-tab = Добавить вкладку
items-links = Ссылки
items-add-link = Добавить ссылку
reveal-mode = Открытие
reveal-fade = Затухание
reveal-fade-help = Верхнее изображение полностью появляется при наведении
reveal-wipe = Шторка
reveal-wipe-help = Верхнее изображение показывается до указателя, как шторка, которая следует за ним
reveal-spot = Прожектор
reveal-spot-help = Верхнее изображение показывается в круге вокруг указателя
reveal-radius = Радиус
reveal-radius-help = Размер прожектора вокруг указателя
reveal-soft = Мягкий край
reveal-soft-help = Ширина затухания на краю прожектора
reveal-under = Нижнее изображение
reveal-over = Открываемое изображение
icons-arrows = Стрелки
icons-interface = Интерфейс
icons-contact = Контакты
icons-media = Медиа
icons-objects = Предметы
icons-design = Дизайн
icons-custom = Импортированные
icons-import = Импортировать значки SVG
accordion-open = Открыт при загрузке
accordion-open-help = Панель открыта при загрузке страницы
accordion-single = Только одна открытая
accordion-single-help = При открытии панели соседние закрываются (Chrome и Edge 120, Firefox 130, Safari 17.2; в других браузерах каждая открывается сама по себе)
accordion-add = Добавить панель
ticker-speed = Скорость
ticker-fade = Затухающие края
anim-speed = Скорость
anim-speed-help = Пикселей в секунду
anim-fade = Затухание
anim-fade-help = Затухание на обоих краях (ширина)
anim-dir-up = Вверх
anim-dir-down = Вниз
filters-role = Фильтры списка
filters-role-help = Кнопки, показывающие только те элементы списка, теги которых содержат их значение (проекты по направлению); без JavaScript всё остаётся на виду.
filters-role-none = Без роли
filters-role-group = Группа фильтров
filters-role-button = Кнопка фильтра
filters-role-item = Фильтруемый элемент
filters-role-empty = Сообщение «Нет результатов»
filters-param = Параметр адреса
filters-param-help = Выбранный фильтр сохраняется в адресе (?expertise=…): ссылка может открывать уже отфильтрованный список. Если пусто: в адресе не сохраняется.
filters-value = Значение
filters-value-help = Значение, которое ищется в тегах элементов; если пусто: показать всё.
filters-value-written = Введено вручную
filters-value-all = Пусто: показать всё
filters-tags = Теги
filters-tags-help = Поле-список записи (направления, категории), значения которого ищут кнопки.
consent-role = Согласие на cookie
consent-role-help = Баннер показывается, пока посетитель не сделает выбор; измерение посещаемости загружается только после согласия. Выбор сохраняется в браузере.
consent-role-none = Без роли
consent-role-banner = Баннер
consent-role-accept = Кнопка «Принять»
consent-role-refuse = Кнопка «Отказаться»
consent-role-open = Ссылка «Управление cookie»
consent-ga = Google Analytics 4
consent-ga-help = Идентификатор измерения (G-…), загружаемый после согласия; если пусто: измерения нет.
tabs-title = Вкладки
tabs-hover = При наведении
tabs-hover-help = Вкладка меняется при наведении и фокусе (список, управляющий фоновым изображением или видео); иначе по нажатию. Видео с автозапуском в показанной панели запускаются.
tabs-keys = Клавиши клавиатуры
tabs-keys-help = Клавиши-стрелки и W, A, S, D переключают вкладки в любом месте страницы (игровое меню, слайд-шоу)
tabs-sound = Звук при смене
tabs-sound-help = Воспроизводится каждый раз, когда активной становится другая вкладка; отключается переключателем «Звуки»
switch-role = Переключатель
switch-role-help = Включает или выключает музыку или звуки сайта (по умолчанию включены, выбор сохраняется в браузере посетителя); состояние «Выкл» оформляется в стилях
switch-none = Нет
switch-music = Музыка
switch-sounds = Звуки
switch-text-on = Текст при включении
switch-text-off = Текст при выключении
switch-text-add = Показывать состояние здесь (Вкл / Выкл)
switch-text-help = Текст этого элемента следует за состоянием содержащего его переключателя
switch-text-remove = Больше не показывать состояние
tabs-role = В вкладках
tabs-role-help = Вкладка: кнопка или ссылка, показывающая свою панель (n-я вкладка показывает n-ю панель). Панель: содержимое, показываемое своей вкладкой.
tabs-role-none = Обычный элемент
tabs-role-tab = Вкладка
tabs-role-panel = Панель
slider-title = Слайды
slider-autoplay = Автозапуск
slider-autoplay-help = Секунд между слайдами; если пусто: вручную
slider-loop = Повтор
insert-accordion = Аккордеон
insert-help-accordion = Заголовок, открывающий или закрывающий своё содержимое, без скрипта
insert-text-accordion = Вопрос
settings-transition = Переход между страницами
settings-transition-help = Переход с одной страницы сайта на другую; в Firefox не работает, отключается для посетителей, которые просят меньше анимации
settings-smooth-scroll = Плавная прокрутка
settings-smooth-scroll-help = Колесо мыши плавно скользит по странице или по прокручиваемой колонке под указателем, а не прыгает на одно деление; отключается для посетителей, которые просят меньше анимации
settings-feeds = Ленты RSS
settings-feeds-help = Одна лента на коллекцию, обновляется при каждой сборке; программы для чтения лент находят там последние записи
settings-feeds-none = Нет коллекции со страницей для каждой записи: лента RSS перечисляет записи, у которых есть собственная страница.
settings-feed-path = Адрес ленты
settings-feed-limit = Количество записей
settings-feed-title = Заголовок
settings-feed-description = Описание
settings-feed-site = Как у сайта
settings-feed-site-desc = Как у сайта
transition-none = Нет
transition-fade = Затухание
transition-slide = Сдвиг
transition-slide-up = Сдвиг вверх
transition-push = Вытеснение
transition-zoom = Масштаб
transition-blur = Размытие
transition-wipe = Шторка
transition-curtain = Занавес
transition-flip = Переворот
help-menu = Справка и сочетания клавиш
help-title = Начало работы
block-list-computed-short = Вычисляемый список
cms-sub-locked-short = Определено в .pages.yml
locales-no-url-short = Нет ссылок между языками: не указан адрес сайта
update-title = Обновления
update-help = Новые версии Revel; перед установкой проверяется их подпись
update-version = Установленная версия
update-check = Проверить обновления
update-install = Установить и перезапустить
update-available = Доступна версия { $version }
update-none = У вас последняя версия Revel
update-disabled = Для этой сборки Revel обновления недоступны
update-pill = Версия { $version }
update-pill-help = Доступна более новая версия Revel: установите её в настройках
update-prereleases = Получать предварительные версии
update-prereleases-help = Предлагаются также тестовые версии, выпускаемые до стабильной; в них могут быть ошибки
update-available-pre = Доступна предварительная версия { $version }
update-pill-pre = Предварительная версия { $version }
styles-truncate = Обрезать
styles-truncate-help = Текст в одну строку, который заканчивается на «…», если не помещается
error-not-a-site = В этой папке нет сайта Revel.
error-trash = Не удалось переместить папку в корзину: { $detail }
site-delete = Удалить сайт…
site-delete-title = Удалить «{ $name }»?
site-delete-body = Папка сайта попадёт в корзину компьютера; оттуда её можно восстановить.
site-delete-confirm = Переместить в корзину
site-thumb = Миниатюра
site-thumb-color = Цвет
site-thumb-homepage = Главная страница
site-thumb-auto = По названию сайта
site-thumb-custom = Другой цвет
site-thumb-retake = Обновить
site-thumb-retake-help = Снять изображение главной страницы заново (также делается само после каждого изменения сайта)
site-thumb-taking = Снимок…
nav-back-to-site = Назад к сайту «{ $name }»
settings-translate-more = Другие языки ({ $count })
settings-translate-less = Меньше языков
settings-translate-none = Ни один язык не установлен
styles-grid = Сетка
grid-columns-count = Колонки
grid-rows-count = Строки
grid-edit = Ширина каждой колонки
grid-flow-row = Заполнять по строкам
grid-flow-column = Заполнять по колонкам
grid-dense = Заполнять пустые ячейки
grid-masonry = Masonry
grid-masonry-help = Masonry: каждый элемент ставится под самую короткую колонку, не оставляя пустых строк (для элементов разной высоты, как в Masonry во Framer)
grid-masonry-order = Колонки по очереди
grid-masonry-order-help = Элемент 1 в первой колонке, 2 во второй и так далее (иначе: в самой короткой колонке)
gap-columns = Колонки
gap-rows = Строки
align-more = Другие варианты выравнивания
align-content-columns = Колонки
align-content-rows = Строки
styles-grid-child = Дочерний элемент сетки
grid-parent-edit = Изменить родительскую сетку
flex-parent-edit = Изменить родительский Flex
child-position = Положение
child-auto = Авто
child-auto-help = Ставится после остальных, занимая выбранное число ячеек
child-manual = Вручную
child-manual-help = Ставится между выбранными линиями сетки
grid-column-line = Линия колонки
grid-row-line = Линия строки
grid-column-start-end = Колонка: начало, конец
grid-row-start-end = Строка: начало, конец
styles-align-self = Выравнивание
styles-justify-self = Выравнивание по ширине
flex-sizing = Размер
order-auto = Нет
order-first = Первый
order-last = Последний
order-custom = Другой порядок (число)

# Importing a site (step 7)
import-title = Импорт сайта
import-source = Источник
import-source-folder = Папка
import-source-folder-help = Папка со страницами HTML, экспорт Webflow (распакованный) или проект Eleventy на этом компьютере; в неё добавляется только файл site.revel.json (и содержимое CMS Webflow)
import-source-git = Репозиторий Git
import-source-git-help = Копирует репозиторий GitHub, любой репозиторий Git по HTTPS или командный репозиторий из общей папки в папку проектов вместе с историей, затем импортирует его
import-folder = Папка
import-choose = Выбрать…
import-url = Адрес репозитория
import-url-placeholder = https://github.com/account/site
import-url-help = github.com/аккаунт/репозиторий, адрес ветки или папки (…/tree/main/docs), любой репозиторий по HTTPS или общая папка команды (NAS, локальная сеть); закрытый репозиторий: подключение к GitHub в настройках
import-git-folder = Папка…
import-git-folder-help = Командный репозиторий в общей папке (NAS, локальная сеть): Настройки › Версии › Командная работа › Общая папка
import-kind-static = Сайт HTML · { $count } { $count ->
        [one] страница
        [few] страницы
        [many] страниц
       *[other] страницы
    }
import-kind-static-help = Страницы, общий макет, таблица стилей на панели «Стили», файлы копируются как есть
import-kind-eleventy = Проект Eleventy
import-kind-build = Сайт, который нужно собрать · { $tool }
import-kind-build-help = Этот сайт создаётся с помощью { $tool }: его страницы получаются при сборке. «Собрать в Revel» запускает сборку на этом компьютере; иначе импортируйте папку с результатом сборки, если она есть в репозитории, или опубликованный сайт
import-kind-unknown = Импортировать нечего
import-kind-unknown-help = В корне нет страниц HTML и нет проекта Eleventy
import-has-site = Уже импортирован
import-no-license = Нет лицензии
import-no-license-help = В этом репозитории нет файла лицензии: сайт остаётся собственностью автора. Убедитесь, что вам разрешено его использовать.
import-output = Импортировать «{ $dir }»
import-output-help = Папка со страницами, найденная в репозитории
import-run = Импортировать
import-cloning = Копирование репозитория…
import-importing = Импорт…
error-import-needs-build = Сайт нужно собрать с помощью { $tool }: импортируйте папку, созданную сборкой (dist, build…).
error-import-nothing = В этой папке нет страниц HTML и проекта Eleventy.
error-git-url = Адрес репозитория не распознан.
error-git-clone-cancelled = Копирование репозитория отменено.
error-not-a-clone = Эта папка не является копией репозитория, которая сейчас импортируется.
import-source-link = Ссылка
import-source-link-help = Опубликованный сайт читается по своему адресу (Framer, Webflow, любой действующий сайт): страницы, стили, изображения и шрифты копируются в папку проектов
import-source-figma = Figma
import-source-figma-help = Фреймы файла Figma как страницы: автолейаут, текст, стили, изображения, компоненты
import-source-wordpress = WordPress
import-source-wordpress-help = Файл экспорта WordPress: страницы, записи, меню и изображения, в шаблоне Revel «Блог»
import-link = Адрес сайта
import-link-placeholder = https://mysite.framer.website
import-link-help = Адрес главной страницы; переход идёт по страницам, на которые ведёт главная, и по карте сайта, правила robots.txt соблюдаются
import-reading = Чтение сайта…
import-progress = Страниц: { $pages } · файлов: { $files } · { $size }
import-made-with = Создано в { $tool }
import-cms = Содержимое CMS
import-cms-csv = Файлы CSV
import-cms-csv-help = Файлы, экспортированные из Webflow (CMS panel › collection › Export), по одному файлу на коллекцию
import-cms-api = API Webflow
import-cms-api-help = Содержимое читается онлайн, включая черновики, с помощью токена сайта: Webflow › Site settings › Apps & integrations › API access › Generate API token (CMS: read). Токен используется только для этого импорта
import-cms-add = Добавить…
import-cms-none = Нет файла
import-cms-token = Токен доступа
import-figma-link = Адрес файла
import-figma-link-placeholder = https://www.figma.com/design/…
import-figma-link-help = Ссылка на файл, скопированная в Figma (Share › Copy link)
import-figma-token = Токен Figma
import-figma-token-help = Figma › Settings › Security › Personal access tokens › Generate new token (File content: read); хранится в связке ключей компьютера
import-figma-token-saved = Токен сохранён
import-figma-forget = Забыть
import-figma-forget-help = Удалить токен Figma из связки ключей компьютера
import-figma-read = Прочитать файл
import-figma-reading = Чтение файла…
import-figma-frames = Фреймы
import-figma-frames-help = Каждый выбранный фрейм становится страницей; первый станет главной
import-figma-all = Все
import-figma-none = Нет
import-wp-file = Файл экспорта
import-wp-file-help = Файл .xml, созданный WordPress: Tools › Export › All content
import-wp-counts = Страниц: { $pages } · записей: { $posts } · медиа: { $media }
import-wp-media = Скачать изображения
import-wp-media-help = Изображения страниц и записей копируются с исходного сайта (из сети); иначе сохраняются исходные адреса
import-downloading = Скачивание… файлов: { $files }
import-done-title = Сайт импортирован
import-stat-pages = Страницы
import-stat-layouts = Макеты
import-stat-styles = Правила стилей
import-stat-animations = Анимации
import-stat-forms = Формы
import-stat-components = Компоненты
import-stat-entries = Записи
import-stat-files = Файлы
import-open = Открыть в редакторе
error-import-url = Адрес сайта не распознан.
error-import-unreachable = Сайт недоступен: { $detail }
error-import-not-html = Этот адрес не ведёт на страницу HTML.
error-import-cancelled = Импорт отменён.
error-import-wxr = { $detail }
error-figma-url = Адрес файла Figma не распознан.
error-figma-token = Токен Figma отсутствует или отклонён.
error-figma-not-found = Файл Figma не найден или недоступен с этим токеном.
error-figma = Figma: { $detail }
error-webflow-token = Токен API Webflow отклонён.
import-build = Собрать в Revel
import-build-help = Revel скачивает инструменты и зависимости проекта, собирает его на этом компьютере, затем импортирует получившиеся страницы
import-build-checking = Проверка нужных инструментов…
import-build-missing-ruby = Для Jekyll на компьютере должны быть установлены Ruby и Bundler (ruby-lang.org): Revel не может установить их сам
import-build-missing-go = Этот сайт на Hugo использует модули: на компьютере должен быть установлен Go (go.dev)
import-build-missing-build-script = В package.json нет скрипта «build»: собирать нечего
import-build-title = Сборка сайта на { $tool }
import-build-risk = Выполняет код проекта
import-build-risk-help = Сборка выполняет на этом компьютере код проекта и его зависимостей, как это делает его автор: только для репозитория, которому вы доверяете
import-build-system = на компьютере
import-build-ready = уже скачано
import-build-download = нужно скачать
import-build-deps = Зависимости ({ $manager })
import-build-lock = { $file } игнорируется
import-build-lock-help = Файл другого менеджера пакетов (Bun): npm сам подбирает версии зависимостей, и они могут отличаться от версий автора
import-build-run = Собрать
import-build-log = Журнал
import-build-step-start = Подготовка…
import-build-step-download = Скачивание { $tool } · { $done } / { $total }
import-build-step-download-start = Скачивание { $tool }…
import-build-step-install = Установка зависимостей ({ $manager })…
import-build-step-build = Сборка сайта ({ $tool })…
import-build-step-serve = Запуск собранного сайта…
import-build-step-read = Чтение страниц · страниц: { $pages } · файлов: { $files }
import-published = Импортировать опубликованный сайт · { $host }
import-published-help = Адрес найден в файлах проекта или на его странице: действующий сайт читается так же, как при источнике «Ссылка»
error-build-download = Не удалось скачать: { $detail }.
error-build-integrity = Скачанный файл изменён, установка отклонена: { $detail }.
error-build-archive = Не удалось прочитать архив: { $detail }.
error-build-platform = Инструмент недоступен для этого компьютера.
error-build-needs-ruby = Для Jekyll на компьютере должны быть установлены Ruby и Bundler.
error-build-needs-go = Для этого сайта на Hugo на компьютере должен быть установлен Go.
error-build-no-script = В package.json нет скрипта «build».
error-build-step = { $step ->
    [install] Установка зависимостей
    [build] Сборка
    [serve] Запуск собранного сайта
   *[other] Этап { $step }
}: сбой (код { $code }).
error-build-timeout = { $step ->
    [install] Установка зависимостей
    [build] Сборка
    [serve] Запуск собранного сайта
   *[other] Этап { $step }
}: превышено время ожидания, выполнение остановлено.
error-build-no-output = Сборка не создала ни одной страницы.
settings-build-tools = Инструменты сборки
settings-build-tools-help = Инструменты, которые Revel скачивает для сборки импортированных сайтов (Node.js, Hugo), и пакеты, сохранённые в кэше для следующих сборок
build-tools-none = Скачанных инструментов нет
build-tools-cache = Пакеты в кэше
build-tools-clear = Очистить
build-tools-remove = Удалить

# Media on hover (video or image that changes with the hovered item)
insert-hover-media = Медиа наведённых элементов
insert-reveal = Открытие при наведении
insert-help-reveal = Второе изображение поверх первого, открываемое при наведении: целиком, до указателя или вокруг него (эскиз, затем готовая работа)
insert-help-hover-media = Видео, показывающее видео наведённого элемента: записи списка CMS (их поле «Видео») или выбранные элементы; настраивается вверху панели «Стили»
insert-progress = Индикатор прогресса
insert-help-progress = Заполняется по мере прокрутки содержащей его колонки или страницы (настраивается в инспекторе); цвет заполнения: цвет текста
inspector-spy = Следует за видимой секцией
inspector-spy-help = Ссылка на читаемую секцию получает класс is-active (состояние «Активен» в стилях); элемент получает атрибут data-revel-section с идентификатором секции.
inspector-progress = Следует за
inspector-progress-help = Прокрутка, заполняющая индикатор: содержащая его колонка (первый прокручиваемый контейнер вокруг него) или вся страница
inspector-progress-parent = Колонка
inspector-progress-page = Страница
hover-title = Медиа при наведении
hover-help = Это видео (или изображение) показывает медиа элемента, на который наведён указатель или который выбран с клавиатуры; при загрузке показывается первое. Показанный элемент получает состояние «Активен», запущенное медиа получает состояние «Медиа готово».
hover-on = Меняется в зависимости от наведённого элемента
hover-on-image = Меняется в зависимости от наведённого элемента
hover-on-help = Показывает видео наведённого элемента (записи списка CMS или выбранные элементы)
hover-source = Наводимые элементы
hover-source-help = Записи списка CMS (каждая показывает своё медиа) или элементы, выбранные по одному
hover-source-list = Записи «{ $list }»
hover-source-manual = Элементы, выбранные по одному
hover-source-manual-help = Выберите элемент на странице, затем укажите это медиа в его настройках в разделе «Медиа при наведении › Показывается в».
hover-none = Нет
hover-field-choose = Выберите поле…
hover-video-field = Видео каждой записи
hover-video-field-help = Поле записи, в котором указан адрес её видео (файл .mp4 из медиа или прямая ссылка)
hover-image-field = Изображение каждой записи
hover-image-field-help = Поле записи, в котором указано её изображение
hover-video-field-none = Ни в одном поле этих записей нет видео. Добавьте его в CMS: файл или текстовое поле с адресом видеофайла (.mp4, .webm).
hover-image-field-none = Ни в одном поле этих записей нет изображения. Добавьте в CMS поле-изображение.
hover-poster-field = Изображение ожидания
hover-poster-help = Показывается, пока загружается видео
hover-items = Меняется при наведении на
hover-item-in-list = { $item } в каждой записи «{ $list }»
hover-select = Выбрать
hover-shown-in = Показывается в
hover-shown-in-help = Медиа при наведении, которое этот элемент меняет, когда на него наводят указатель
hover-missing = Медиа при наведении, которое менял этот элемент, больше не существует.
hover-list-note = Каждая запись списка показывает своё медиа: измените его в CMS.
hover-video-src = Видео (адрес)
hover-image-src = Изображение (адрес)
hover-poster-src = Изображение ожидания (адрес)
inspector-src-video = Видео (адрес)
inspector-src-audio = Аудио (адрес)
styles-state-ready = Медиа готово
styles-state-in-paused = Видео приостановлено
tabs-pair-tab = Вкладка { $n } из { $count }: показывает
tabs-pair-panel = Панель { $n } из { $count }: показывается вкладкой
tabs-pair-none = Нет панели: панелей меньше, чем вкладок.
tabs-pair-repeated = Вкладки или панели повторяются списком: n-я запись показывает n-ю панель.
tabs-pair-mismatch = Вкладок: { $tabs }, панелей: { $panels }. N-я вкладка показывает n-ю панель, поэтому с обеих сторон должно быть поровну.

# Site information (contact details, social links, menus)
siteinfo-rename = Переименовать
siteinfo-rename-help = Двойной щелчок, чтобы переименовать
siteinfo-open-link = Открыть ссылку
siteinfo-title = Информация о сайте
siteinfo-help = Значения, записанные один раз и используемые на разных страницах: изменённые здесь, они меняются везде. Свяжите текст, ссылку или изображение со значением через «Стили › Информация о сайте».
siteinfo-none = Информации пока нет.
siteinfo-label = Отображаемое название
siteinfo-uses = { $count ->
    [0] Не используется
    [one] Используется { $count } раз
   *[other] Используется { $count } раз
}
siteinfo-uses-help = Количество мест на сайте (страницы, макеты, компоненты), где используется это значение; используемое значение удалить нельзя
siteinfo-new-label = Название (например, «Часы работы»)
siteinfo-new-value = Значение
siteinfo-add = Добавить
siteinfo-bind = Информация о сайте
siteinfo-bind-help = Текст, ссылка или изображение берутся из значения сайта (Настройки › Информация о сайте): изменение один раз действует везде
siteinfo-year = Текущий год
siteinfo-link-written = Введена вручную
siteinfo-link-mail = { $name } (написать письмо)
siteinfo-link-tel = { $name } (позвонить)
siteinfo-image-written = Выбрано вручную
siteinfo-edit = Изменить информацию о сайте
siteinfo-key-name = Название
siteinfo-key-title = Заголовок
siteinfo-key-description = Описание
siteinfo-key-url = Адрес сайта
siteinfo-key-locale = Язык (регион)
siteinfo-key-lang = Язык
siteinfo-key-address = Адрес
siteinfo-key-street = Улица
siteinfo-key-postcode = Почтовый индекс
siteinfo-key-city = Город
siteinfo-key-country = Страна
siteinfo-key-phone = Телефон
siteinfo-key-fax = Факс
siteinfo-key-email = Эл. почта
siteinfo-key-social = Социальные сети
siteinfo-key-facebook = Facebook
siteinfo-key-linkedin = LinkedIn
siteinfo-key-instagram = Instagram
siteinfo-key-youtube = YouTube
siteinfo-key-twitter = X (Twitter)
siteinfo-key-x = X
siteinfo-key-tiktok = TikTok
siteinfo-key-ga4 = Google Analytics
siteinfo-key-share_image = Изображение для соцсетей
siteinfo-key-menus = Меню
siteinfo-key-contact_cta = Призыв связаться
siteinfo-key-subtitle = Подзаголовок
siteinfo-key-text = Текст
siteinfo-key-button = Кнопка
siteinfo-key-contact_form = Контактная форма
siteinfo-key-recipient = Получатель
siteinfo-key-subject = Тема
siteinfo-key-success = Сообщение отправлено
siteinfo-key-error = Сообщение об ошибке
sitemenus-title = Меню сайта
sitemenus-help = Списки ссылок, используемые меню страниц (шапка, мобильное меню, подвал): ссылка, изменённая здесь, меняется везде
sitemenus-none = Меню нет: выберите на странице список ссылок, затем нажмите «Создать меню из этих ссылок» в панели «Стили».
sitemenus-name = Название меню
sitemenus-remove = Удалить это меню (только когда оно больше не используется)
sitemenus-new = Название нового меню
sitemenus-add = Новое меню
sitemenus-list = Меню сайта
sitemenus-links-of = Ссылки меню
sitemenus-edit = Изменить ссылки (Настройки › Навигация)
sitemenus-written = Ссылок, введённых вручную: { $count }
sitemenus-create = Создать меню из этих ссылок
sitemenus-link = Связать с меню
sitemenus-link-help = Введённые вручную ссылки, которые есть в этом меню, заменяются списком меню; остальные остаются введёнными вручную
sitemenus-item = Ссылка меню «{ $menu }»
sitemenus-item-help = Каждая ссылка меню повторяет этот элемент со своим текстом и адресом
sitemenus-item-label = Текст каждой ссылки
sitemenus-item-url = Адрес каждой ссылки

# Part roles (carousel, video player, form)
role-slider = В карусели
role-slider-help = Роль элемента в содержащей его карусели или слайд-шоу
role-slider-track = Лента (прокручивается)
role-slider-prev = Кнопка «Назад»
role-slider-next = Кнопка «Вперёд»
role-slider-count = Счётчик (1 / 5)
role-slider-dots = Точки (создаются сайтом)
role-player = В видеоплеере
role-player-help = Роль элемента в содержащем его видеоплеере
role-player-cover = Обложка (запускает видео)
role-cart = Корзина
role-cart-help = Корзина сайта: товары, добавленные кнопками «В корзину», хранятся в браузере посетителя, заказ отправляется формой корзины
role-cart-currency = Валюта
role-cart-currency-help = Валюта цен и итога (трёхбуквенный код)
role-cart-item = Строка товара (шаблон)
role-cart-name = Название товара
role-cart-qty = Количество
role-cart-price = Цена строки
role-cart-plus = Кнопка «+»
role-cart-minus = Кнопка «−»
role-cart-remove = Кнопка «Удалить»
role-cart-total = Итого
role-cart-empty = Сообщение «Корзина пуста»
role-cart-field = Поле, отправляемое с заказом
role-cart-add = В корзину
role-cart-add-help = Кнопка, добавляющая товар в корзину: товар и цена (вводятся вручную или берутся из поля записи)
role-cart-product = Товар
role-cart-product-help = Название товара в корзине и в заказе
role-cart-price-value = Цена
role-cart-price-help = Цена одного товара (число, без валюты)
role-cart-count = Количество товаров в корзине
role-cart-count-help = Показывает количество товаров в корзине (панель навигации)
role-remove = Убрать эту роль
role-form = Форма сайта
role-form-help = Форма, отправляемая по электронной почте вашим хостингом (Настройки › Формы)
role-form-send = Отправляется сайтом
role-form-send-help = Поля отправляются по электронной почте на адрес, указанный в разделе «Настройки › Формы»
role-form-name = Название формы
role-form-name-help = Название, используемое в теме письма («contact»)
role-form-settings = Настройки формы
role-form-status = Сообщение после отправки

## Valeurs reliées des attributs
bound-value = Значение
bound-before = Текст перед
bound-after = Текст после
bound-only-if = Только если значение заполнено
bound-only-if-help = Текст перед, значение и текст после записываются, только если значение заполнено; постоянные тексты записываются всегда («Смотреть видео: Заголовок», иначе «Смотреть видео»).
bound-fixed = Постоянный текст перед
bound-tail = Постоянный текст после
bound-link = Связать со значением (поле, свойство, информация о сайте)
bound-unlink = Разорвать связь: вместо неё введённый текст
bound-count = Количество записей › { $collection }
bound-style-prop = Свойство стиля
bound-style-remove = Удалить свойство
bound-style-add = Связанное свойство
condition-field-page = Страница записи (у записи есть страница)
block-bound-format = Формат
block-bound-format-help = Как записывается значение: дата или формат сайта (Настройки › Фильтры).
block-bound-join = Значения «{ $field }»
block-bound-join-sep = Разделитель
block-bound-fallback-text = Текст, если пусто
block-bound-fallback-text-help = Записывается, когда поле (и запасное поле) пусто.
condition-field-first = Первый из списка
condition-field-last = Последний из списка
bound-fallback = Если пусто
bound-fallback-help = Текст, записываемый, когда значение пусто.
role-picker = Роль
role-picker-help = Что этот элемент делает в окружающем его интерактивном элементе или на странице (вкладка, стрелка карусели, кнопка фильтра, кнопка согласия, переключатель звука…).
pages-file-row-help = Серверный файл: его текст изменяется в его настройках
canvas-render-error = Страницу больше нельзя отобразить после этого изменения (остаётся показанной предыдущая версия; Ctrl+Z отменяет изменение): { $error }
# Plan du site (D-103)
map-search = Найти страницу
map-shared = Общие части
map-shared-help = Меню сайта, компоненты и шаблоны, ссылки которых появляются на нескольких страницах
map-external = Другие сайты
map-external-help = Ссылки на другие сайты, сгруппированные по доменам
map-count-pages = { $count ->
    [one] { $count } страница
    [few] { $count } страницы
    [many] { $count } страниц
   *[other] { $count } страницы
}
map-count-broken = { $count ->
    [one] { $count } адрес без страницы
    [few] { $count } адреса без страницы
    [many] { $count } адресов без страницы
   *[other] { $count } адреса без страницы
}
map-count-orphans = { $count ->
    [one] { $count } страница без ссылок на неё
    [few] { $count } страницы без ссылок на них
    [many] { $count } страниц без ссылок на них
   *[other] { $count } страницы без ссылок на них
}
map-broken-help = Ссылки ведут на адрес сайта, у которого нет страницы (красным): создайте страницу, настройте перенаправление с этого адреса или измените ссылки
map-orphan-help = Ни одна ссылка не ведёт на эту страницу: посетители попадают на неё только по адресу
map-media = Медиа и соцсети
map-media-help = Видео (YouTube, Vimeo…), аудио и социальные сети, используемые на сайте, сгруппированные по платформам; их адреса проверяются
map-kind-media = Видео и аудио
map-kind-social = Социальная сеть
map-urls-count = { $count ->
    [one] { $count } адрес
    [few] { $count } адреса
    [many] { $count } адресов
   *[other] { $count } адреса
}
map-urls = Адреса ({ $count })
map-urls-check = Проверить
map-urls-check-help = Проверить все адреса заново (результаты сохраняются на сутки)
map-urls-checking = Проверка адресов…
map-urls-review = { $count ->
    [one] { $count } адрес требует проверки
    [few] { $count } адреса требуют проверки
    [many] { $count } адресов требуют проверки
   *[other] { $count } адреса требуют проверки
}
map-urls-review-help = Адреса других сайтов, которые не найдены, не отвечают, закрыты или перенесены: показать другие сайты и медиа
map-url-ok = Доступен
map-url-redirected = Перенесён
map-url-missing = Не найден
map-url-restricted = Закрыт
map-url-blocked = Проверить нельзя
map-url-failing = Ошибка
map-url-unreachable = Нет ответа
map-url-unchecked = Не проверен
map-url-final = Ведёт на
map-url-use-final = Использовать этот адрес
map-url-open = Открыть в браузере
map-url-replace = Заменить
map-url-replace-help = Заменить этот адрес везде, где он используется (страницы, медиа, меню, записи CMS, информация о сайте)
map-url-replace-title = Заменить адрес везде
map-url-new = Новый адрес
map-url-replaced = { $count ->
    [one] Адрес заменён в { $count } месте: { $url }
    [few] Адрес заменён в { $count } местах: { $url }
    [many] Адрес заменён в { $count } местах: { $url }
   *[other] Адрес заменён в { $count } местах: { $url }
}
map-url-uses = { $count ->
    [one] Используется в { $count } месте
    [few] Используется в { $count } местах
    [many] Используется в { $count } местах
   *[other] Используется в { $count } местах
}
map-source-info = Информация о сайте › { $name }
map-source-cms = CMS › { $name }
map-via-src = Встроенное медиа
map-via-field = Поле «{ $field }»
map-via-body = Текст CMS
map-via-info = Информация о сайте
map-arrange = Упорядочить
map-arrange-help = Вернуть все узлы в порядок адресов, включая перемещённые (Ctrl+Z отменяет)
map-show-in-editor = Показать в редакторе
map-retargeted = { $count ->
    [one] { $count } ссылка теперь ведёт на { $to }
    [few] { $count } ссылки теперь ведут на { $to }
    [many] { $count } ссылок теперь ведут на { $to }
   *[other] { $count } ссылки теперь ведут на { $to }
}
map-linked-menu = Ссылка на { $to } добавлена в меню
map-linked-page = Ссылка на { $to } добавлена в конец страницы
map-entries = { $count ->
    [0] Нет записей
    [one] { $count } запись
    [few] { $count } записи
    [many] { $count } записей
   *[other] { $count } записи
}
map-entries-help = Показать или скрыть записи коллекции на схеме
map-entries-show = Показать записи
map-entries-hide = Скрыть записи
map-draft = Черновик
map-draft-help = Страница-черновик: не публикуется
map-port-help = Перетащите на страницу, чтобы добавить ссылку на неё
map-menu-links = { $count ->
    [one] { $count } ссылка
    [few] { $count } ссылки
    [many] { $count } ссылок
   *[other] { $count } ссылки
}
map-shared-pages = { $count ->
    [0] Ни на одной странице
    [one] На { $count } странице
    [few] На { $count } страницах
    [many] На { $count } страницах
   *[other] На { $count } страницах
}
map-minimap = Обзор: щёлкните или перетащите, чтобы сдвинуть вид
map-zoom-out = Уменьшить (−)
map-zoom-in = Увеличить (+)
map-zoom-reset = Реальный размер (0)
map-fit = Показать всю схему (F)
map-key-link = Ссылка
map-key-list = Список записей
map-key-menu = Меню
map-key-redirect = Перенаправление
map-key-broken = Нет страницы
map-panel = Выделение
map-edge = Ссылки
map-retarget = Направить на
map-retarget-help = Изменить, куда ведут эти ссылки (введённые вручную ссылки, ссылки меню, перенаправления; список следует за своей коллекцией). Конец выбранной линии можно также перетащить на другую страницу.
map-retarget-choose = Выберите страницу…
map-links-count = { $count ->
    [one] { $count } ссылка
    [few] { $count } ссылки
    [many] { $count } ссылок
   *[other] { $count } ссылки
}
map-links-in = Ссылки на неё ({ $count })
map-links-out = Ссылки с неё ({ $count })
map-none = Нет
map-via-list = Список записей
map-via-menu = Меню сайта
map-via-menu-item = Ссылка меню
map-via-redirect = Перенаправление
map-via-arg = Значение компонента
map-redirected-help = Адрес, указанный в ссылке, перенаправляет (301) на эту страницу
map-kind-page = Страница
map-kind-collection = Коллекция
map-kind-entry = Запись
map-kind-layout = Шаблон
map-kind-component = Компонент
map-kind-menu = Меню сайта
map-kind-missing = Адрес без страницы
map-kind-external = Другой сайт
map-page-name = Название
map-page-path = Адрес
map-page-path-help = При смене адреса опубликованной страницы добавляется перенаправление со старого
map-page-path-taken = Недопустимый адрес или он уже используется другой страницей
map-open = Открыть в редакторе
map-new-child = Новая вложенная страница
map-open-entry = Изменить запись
map-open-template = Изменить шаблон
map-open-template-help = Открывает в редакторе страницу первой записи: изменения применяются ко всем записям
map-open-cms = Открыть в CMS
map-edit-menus = Изменить меню
map-menu-help = Перетащите его точку на страницу, чтобы добавить ссылку на это меню.
map-shared-note = Его ссылки есть на каждой странице, где оно показано.
map-missing-help = Ссылки ведут на этот адрес, но такой страницы нет.
map-missing-redirected = Этот старый адрес перенаправляет на другую страницу.
map-create-page = Создать эту страницу
map-redirect = Перенаправить на
map-redirect-help = Постоянное перенаправление (301) с этого адреса на страницу
map-open-site = Открыть сайт
view-map = Схема сайта

# Paramètres (D-104)
settings-general-lead = Название, публичный адрес и язык сайта, ленты RSS, структура и окружения.
settings-seo-lead = Карта сайта, robots.txt, изображение для соцсетей и организация: то, что читают поисковые системы и социальные сети.
settings-publish-lead = Где публикуется сайт (SFTP-хостинг): проверка и отправка выполняются отсюда.
settings-app-general-lead = Язык, тема и цвет Revel, а также папка новых сайтов.
settings-accounts-lead = Ваше фото, которое показывается вверху справа, ваше имя, записываемое в сохраняемых вами версиях, и аккаунты, которые использует Revel.
siteinfo-remove-used = Всё ещё используется: сначала отвяжите его там, где оно используется

# Profil (D-106)
profile-title = Профиль
profile-picture = Фото
profile-picture-help = Показывается вверху справа. Если собственного фото нет, используется фото подключённого аккаунта GitHub.
profile-picture-change = Изменить фото
profile-picture-remove = Удалить
profile-picture-github = Использовать фото с GitHub
settings-profile = Профиль и аккаунты
error-profile-picture = Не удаётся прочитать это изображение: выберите файл PNG, JPEG, WebP, GIF или AVIF.
