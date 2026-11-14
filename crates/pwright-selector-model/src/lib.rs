//! Command-free, backend-neutral lazy selector values for Pwright.
//!
//! Selector values are immutable recipes. They contain no command identity,
//! transport, runtime, browser backend, or DOM/browser-protocol handle.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU32;
use std::str::FromStr;

/// Stable selector/action failure details shared by command capabilities and
/// command-free backend selector plugins.
///
/// Keeping this value beside [`Selector`] lets an internal selector runtime be
/// reused without importing the public `pwright.locator` command catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocatorErrorDetails {
    InvalidSelector(String),
    StrictViolation { selector: String, matches: u32 },
    InvalidTarget(String),
    InvalidValue(String),
    InvalidBackendResponse(String),
    Backend(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CssSelector(String);

impl CssSelector {
    pub fn new(value: impl Into<String>) -> Result<Self, CssSelectorError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CssSelectorError);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Allocated string capacity retained by this selector leaf.
    ///
    /// Backends use this for resource accounting; portable callers normally
    /// care only about [`Self::as_str`].
    pub fn retained_string_capacity(&self) -> usize {
        self.0.capacity()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CssSelectorError;

impl Display for CssSelectorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("CSS selector must not be empty")
    }
}

impl Error for CssSelectorError {}

/// String matching used by Playwright's user-facing locator engines.
///
/// Both modes normalize whitespace. Exact matching is case-sensitive and
/// whole-string; non-exact matching is a case-insensitive substring match.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextMatcher {
    value: String,
    exact: bool,
}

impl TextMatcher {
    pub fn new(value: impl Into<String>, exact: bool) -> Self {
        Self {
            value: value.into(),
            exact,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// Allocated string capacity retained by this matcher.
    pub fn retained_string_capacity(&self) -> usize {
        self.value.capacity()
    }

    pub const fn exact(&self) -> bool {
        self.exact
    }
}

/// A JavaScript-compatible regular expression used by Playwright-shaped
/// locator APIs.
///
/// Flags use the canonical order returned by `RegExp.prototype.flags`:
/// `d`, `g`, `i`, `m`, `s`, `u`, `v`, `y`. Duplicate, out-of-order, unknown,
/// and mutually exclusive `u`/`v` flags are rejected before a command can be
/// encoded. The expression source itself is compiled by the browser so its
/// ECMAScript version remains a backend capability rather than a Rust-regex
/// dependency in the portable crate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegexMatcher {
    source: String,
    flags: String,
}

impl RegexMatcher {
    pub fn new(
        source: impl Into<String>,
        flags: impl Into<String>,
    ) -> Result<Self, RegexMatcherError> {
        let source = source.into();
        let flags = flags.into();
        validate_regex_flags(&flags)?;
        Ok(Self { source, flags })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn flags(&self) -> &str {
        &self.flags
    }

    /// Allocated string capacity retained by this matcher.
    pub fn retained_string_capacity(&self) -> usize {
        self.source.capacity().saturating_add(self.flags.capacity())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegexMatcherError {
    UnsupportedFlag(char),
    DuplicateFlag(char),
    NonCanonicalOrder,
    ConflictingUnicodeModes,
}

impl Display for RegexMatcherError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFlag(flag) => {
                write!(
                    formatter,
                    "unsupported ECMAScript regular-expression flag `{flag}`"
                )
            }
            Self::DuplicateFlag(flag) => {
                write!(
                    formatter,
                    "duplicate ECMAScript regular-expression flag `{flag}`"
                )
            }
            Self::NonCanonicalOrder => formatter.write_str(
                "ECMAScript regular-expression flags must use canonical `dgimsuvy` order",
            ),
            Self::ConflictingUnicodeModes => formatter.write_str(
                "ECMAScript regular-expression flags `u` and `v` are mutually exclusive",
            ),
        }
    }
}

impl Error for RegexMatcherError {}

fn validate_regex_flags(flags: &str) -> Result<(), RegexMatcherError> {
    const ORDER: &str = "dgimsuvy";
    let mut previous = None;
    let mut unicode = false;
    let mut unicode_sets = false;
    for flag in flags.chars() {
        let position = ORDER
            .find(flag)
            .ok_or(RegexMatcherError::UnsupportedFlag(flag))?;
        if previous == Some(position) {
            return Err(RegexMatcherError::DuplicateFlag(flag));
        }
        if previous.is_some_and(|previous| previous > position) {
            return Err(RegexMatcherError::NonCanonicalOrder);
        }
        previous = Some(position);
        unicode |= flag == 'u';
        unicode_sets |= flag == 'v';
    }
    if unicode && unicode_sets {
        return Err(RegexMatcherError::ConflictingUnicodeModes);
    }
    Ok(())
}

/// Fixed user-facing attributes supported by Playwright's convenience
/// locators. Test-id attributes are represented separately because callers may
/// configure one or more names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserFacingAttribute {
    Placeholder,
    Alt,
    Title,
}

impl UserFacingAttribute {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Placeholder => "placeholder",
            Self::Alt => "alt",
            Self::Title => "title",
        }
    }

    pub const fn method_name(self) -> &'static str {
        match self {
            Self::Placeholder => "placeholder",
            Self::Alt => "alt_text",
            Self::Title => "title",
        }
    }
}

pub const MAX_TEST_ID_ATTRIBUTES: usize = 16;

/// Canonical comma-separated test-id attribute names carried by a locator.
///
/// Playwright supports comma-separated names through its mutable selector
/// registry. Pwright stores the same selection in the immutable query value so
/// independently composed clients cannot race through process-global state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestIdAttributes(String);

impl TestIdAttributes {
    pub fn new(value: impl Into<String>) -> Result<Self, TestIdAttributesError> {
        let value = value.into();
        let mut count = 0;
        for name in value.split(',') {
            count += 1;
            if count > MAX_TEST_ID_ATTRIBUTES {
                return Err(TestIdAttributesError::TooMany {
                    actual: count,
                    limit: MAX_TEST_ID_ATTRIBUTES,
                });
            }
            if name.is_empty() {
                return Err(TestIdAttributesError::EmptyName { index: count - 1 });
            }
            if let Some(character) = name.chars().find(|character| {
                character.is_control()
                    || character.is_ascii_whitespace()
                    || matches!(character, '"' | '\'' | '>' | '/' | '=')
            }) {
                return Err(TestIdAttributesError::InvalidCharacter {
                    index: count - 1,
                    character,
                });
            }
        }
        Ok(Self(value))
    }

    pub fn standard() -> Self {
        Self("data-testid".to_string())
    }

    pub fn as_csv(&self) -> &str {
        &self.0
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.split(',')
    }

    pub fn retained_string_capacity(&self) -> usize {
        self.0.capacity()
    }
}

impl Default for TestIdAttributes {
    fn default() -> Self {
        Self::standard()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestIdAttributesError {
    EmptyName { index: usize },
    TooMany { actual: usize, limit: usize },
    InvalidCharacter { index: usize, character: char },
}

impl Display for TestIdAttributesError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName { index } => {
                write!(formatter, "test-id attribute name {index} is empty")
            }
            Self::TooMany { actual, limit } => write!(
                formatter,
                "test-id attribute count {actual} exceeds limit {limit}"
            ),
            Self::InvalidCharacter { index, character } => write!(
                formatter,
                "test-id attribute name {index} contains invalid character {character:?}"
            ),
        }
    }
}

impl Error for TestIdAttributesError {}

/// ARIA roles accepted by Playwright's `get_by_role` contract.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AriaRole {
    Alert,
    AlertDialog,
    Application,
    Article,
    Banner,
    Blockquote,
    Button,
    Caption,
    Cell,
    Checkbox,
    Code,
    ColumnHeader,
    Combobox,
    Complementary,
    ContentInfo,
    Definition,
    Deletion,
    Dialog,
    Directory,
    Document,
    Emphasis,
    Feed,
    Figure,
    Form,
    Generic,
    Grid,
    GridCell,
    Group,
    Heading,
    Img,
    Insertion,
    Link,
    List,
    Listbox,
    ListItem,
    Log,
    Main,
    Marquee,
    Math,
    Meter,
    Menu,
    Menubar,
    MenuItem,
    MenuItemCheckbox,
    MenuItemRadio,
    Navigation,
    None,
    Note,
    Option,
    Paragraph,
    Presentation,
    Progressbar,
    Radio,
    RadioGroup,
    Region,
    Row,
    RowGroup,
    RowHeader,
    Scrollbar,
    Search,
    Searchbox,
    Separator,
    Slider,
    Spinbutton,
    Status,
    Strong,
    Subscript,
    Superscript,
    Switch,
    Tab,
    Table,
    Tablist,
    Tabpanel,
    Term,
    Textbox,
    Time,
    Timer,
    Toolbar,
    Tooltip,
    Tree,
    Treegrid,
    Treeitem,
}

impl AriaRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Alert => "alert",
            Self::AlertDialog => "alertdialog",
            Self::Application => "application",
            Self::Article => "article",
            Self::Banner => "banner",
            Self::Blockquote => "blockquote",
            Self::Button => "button",
            Self::Caption => "caption",
            Self::Cell => "cell",
            Self::Checkbox => "checkbox",
            Self::Code => "code",
            Self::ColumnHeader => "columnheader",
            Self::Combobox => "combobox",
            Self::Complementary => "complementary",
            Self::ContentInfo => "contentinfo",
            Self::Definition => "definition",
            Self::Deletion => "deletion",
            Self::Dialog => "dialog",
            Self::Directory => "directory",
            Self::Document => "document",
            Self::Emphasis => "emphasis",
            Self::Feed => "feed",
            Self::Figure => "figure",
            Self::Form => "form",
            Self::Generic => "generic",
            Self::Grid => "grid",
            Self::GridCell => "gridcell",
            Self::Group => "group",
            Self::Heading => "heading",
            Self::Img => "img",
            Self::Insertion => "insertion",
            Self::Link => "link",
            Self::List => "list",
            Self::Listbox => "listbox",
            Self::ListItem => "listitem",
            Self::Log => "log",
            Self::Main => "main",
            Self::Marquee => "marquee",
            Self::Math => "math",
            Self::Meter => "meter",
            Self::Menu => "menu",
            Self::Menubar => "menubar",
            Self::MenuItem => "menuitem",
            Self::MenuItemCheckbox => "menuitemcheckbox",
            Self::MenuItemRadio => "menuitemradio",
            Self::Navigation => "navigation",
            Self::None => "none",
            Self::Note => "note",
            Self::Option => "option",
            Self::Paragraph => "paragraph",
            Self::Presentation => "presentation",
            Self::Progressbar => "progressbar",
            Self::Radio => "radio",
            Self::RadioGroup => "radiogroup",
            Self::Region => "region",
            Self::Row => "row",
            Self::RowGroup => "rowgroup",
            Self::RowHeader => "rowheader",
            Self::Scrollbar => "scrollbar",
            Self::Search => "search",
            Self::Searchbox => "searchbox",
            Self::Separator => "separator",
            Self::Slider => "slider",
            Self::Spinbutton => "spinbutton",
            Self::Status => "status",
            Self::Strong => "strong",
            Self::Subscript => "subscript",
            Self::Superscript => "superscript",
            Self::Switch => "switch",
            Self::Tab => "tab",
            Self::Table => "table",
            Self::Tablist => "tablist",
            Self::Tabpanel => "tabpanel",
            Self::Term => "term",
            Self::Textbox => "textbox",
            Self::Time => "time",
            Self::Timer => "timer",
            Self::Toolbar => "toolbar",
            Self::Tooltip => "tooltip",
            Self::Tree => "tree",
            Self::Treegrid => "treegrid",
            Self::Treeitem => "treeitem",
        }
    }
}

impl Display for AriaRole {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AriaRoleParseError;

impl Display for AriaRoleParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("unsupported ARIA role")
    }
}

impl Error for AriaRoleParseError {}

impl FromStr for AriaRole {
    type Err = AriaRoleParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "alert" => Ok(Self::Alert),
            "alertdialog" => Ok(Self::AlertDialog),
            "application" => Ok(Self::Application),
            "article" => Ok(Self::Article),
            "banner" => Ok(Self::Banner),
            "blockquote" => Ok(Self::Blockquote),
            "button" => Ok(Self::Button),
            "caption" => Ok(Self::Caption),
            "cell" => Ok(Self::Cell),
            "checkbox" => Ok(Self::Checkbox),
            "code" => Ok(Self::Code),
            "columnheader" => Ok(Self::ColumnHeader),
            "combobox" => Ok(Self::Combobox),
            "complementary" => Ok(Self::Complementary),
            "contentinfo" => Ok(Self::ContentInfo),
            "definition" => Ok(Self::Definition),
            "deletion" => Ok(Self::Deletion),
            "dialog" => Ok(Self::Dialog),
            "directory" => Ok(Self::Directory),
            "document" => Ok(Self::Document),
            "emphasis" => Ok(Self::Emphasis),
            "feed" => Ok(Self::Feed),
            "figure" => Ok(Self::Figure),
            "form" => Ok(Self::Form),
            "generic" => Ok(Self::Generic),
            "grid" => Ok(Self::Grid),
            "gridcell" => Ok(Self::GridCell),
            "group" => Ok(Self::Group),
            "heading" => Ok(Self::Heading),
            "img" => Ok(Self::Img),
            "insertion" => Ok(Self::Insertion),
            "link" => Ok(Self::Link),
            "list" => Ok(Self::List),
            "listbox" => Ok(Self::Listbox),
            "listitem" => Ok(Self::ListItem),
            "log" => Ok(Self::Log),
            "main" => Ok(Self::Main),
            "marquee" => Ok(Self::Marquee),
            "math" => Ok(Self::Math),
            "meter" => Ok(Self::Meter),
            "menu" => Ok(Self::Menu),
            "menubar" => Ok(Self::Menubar),
            "menuitem" => Ok(Self::MenuItem),
            "menuitemcheckbox" => Ok(Self::MenuItemCheckbox),
            "menuitemradio" => Ok(Self::MenuItemRadio),
            "navigation" => Ok(Self::Navigation),
            "none" => Ok(Self::None),
            "note" => Ok(Self::Note),
            "option" => Ok(Self::Option),
            "paragraph" => Ok(Self::Paragraph),
            "presentation" => Ok(Self::Presentation),
            "progressbar" => Ok(Self::Progressbar),
            "radio" => Ok(Self::Radio),
            "radiogroup" => Ok(Self::RadioGroup),
            "region" => Ok(Self::Region),
            "row" => Ok(Self::Row),
            "rowgroup" => Ok(Self::RowGroup),
            "rowheader" => Ok(Self::RowHeader),
            "scrollbar" => Ok(Self::Scrollbar),
            "search" => Ok(Self::Search),
            "searchbox" => Ok(Self::Searchbox),
            "separator" => Ok(Self::Separator),
            "slider" => Ok(Self::Slider),
            "spinbutton" => Ok(Self::Spinbutton),
            "status" => Ok(Self::Status),
            "strong" => Ok(Self::Strong),
            "subscript" => Ok(Self::Subscript),
            "superscript" => Ok(Self::Superscript),
            "switch" => Ok(Self::Switch),
            "tab" => Ok(Self::Tab),
            "table" => Ok(Self::Table),
            "tablist" => Ok(Self::Tablist),
            "tabpanel" => Ok(Self::Tabpanel),
            "term" => Ok(Self::Term),
            "textbox" => Ok(Self::Textbox),
            "time" => Ok(Self::Time),
            "timer" => Ok(Self::Timer),
            "toolbar" => Ok(Self::Toolbar),
            "tooltip" => Ok(Self::Tooltip),
            "tree" => Ok(Self::Tree),
            "treegrid" => Ok(Self::Treegrid),
            "treeitem" => Ok(Self::Treeitem),
            _ => Err(AriaRoleParseError),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AriaTriState {
    False,
    True,
    Mixed,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RoleOptions {
    pub name: Option<TextMatcher>,
    pub description: Option<TextMatcher>,
    pub name_regex: Option<RegexMatcher>,
    pub description_regex: Option<RegexMatcher>,
    pub checked: Option<AriaTriState>,
    pub disabled: Option<bool>,
    pub expanded: Option<bool>,
    pub include_hidden: bool,
    pub level: Option<NonZeroU32>,
    pub pressed: Option<AriaTriState>,
    pub selected: Option<bool>,
}

impl RoleOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, value: impl Into<String>, exact: bool) -> Self {
        self.name = Some(TextMatcher::new(value, exact));
        self.name_regex = None;
        self
    }

    pub fn with_description(mut self, value: impl Into<String>, exact: bool) -> Self {
        self.description = Some(TextMatcher::new(value, exact));
        self.description_regex = None;
        self
    }

    pub fn with_name_regex(mut self, matcher: RegexMatcher) -> Self {
        self.name = None;
        self.name_regex = Some(matcher);
        self
    }

    pub fn with_description_regex(mut self, matcher: RegexMatcher) -> Self {
        self.description = None;
        self.description_regex = Some(matcher);
        self
    }

    pub const fn with_checked(mut self, value: AriaTriState) -> Self {
        self.checked = Some(value);
        self
    }

    pub const fn with_disabled(mut self, value: bool) -> Self {
        self.disabled = Some(value);
        self
    }

    pub const fn with_expanded(mut self, value: bool) -> Self {
        self.expanded = Some(value);
        self
    }

    pub const fn with_include_hidden(mut self, value: bool) -> Self {
        self.include_hidden = value;
        self
    }

    pub const fn with_level(mut self, value: NonZeroU32) -> Self {
        self.level = Some(value);
        self
    }

    pub const fn with_pressed(mut self, value: AriaTriState) -> Self {
        self.pressed = Some(value);
        self
    }

    pub const fn with_selected(mut self, value: bool) -> Self {
        self.selected = Some(value);
        self
    }

    fn uses_regex(&self) -> bool {
        self.name_regex.is_some() || self.description_regex.is_some()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleSelector {
    role: AriaRole,
    options: RoleOptions,
}

impl RoleSelector {
    pub fn new(role: AriaRole, options: RoleOptions) -> Result<Self, RoleSelectorError> {
        validate_role_options(role, &options)?;
        if options.uses_regex() {
            return Err(RoleSelectorError {
                option: "regular-expression matcher",
                role,
            });
        }
        Ok(Self { role, options })
    }

    pub const fn role(&self) -> AriaRole {
        self.role
    }

    pub const fn options(&self) -> &RoleOptions {
        &self.options
    }
}

/// Role selector representation used only when at least one accessible name
/// or description matcher is a regular expression. Keeping this separate from
/// [`RoleSelector`] lets protocol 1.4 append a new selector tag without
/// changing the established 1.3 role bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RolePatternSelector {
    role: AriaRole,
    options: RoleOptions,
}

impl RolePatternSelector {
    pub fn new(role: AriaRole, options: RoleOptions) -> Result<Self, RoleSelectorError> {
        validate_role_options(role, &options)?;
        if !options.uses_regex() {
            return Err(RoleSelectorError {
                option: "missing regular-expression matcher",
                role,
            });
        }
        Ok(Self { role, options })
    }

    pub const fn role(&self) -> AriaRole {
        self.role
    }

    pub const fn options(&self) -> &RoleOptions {
        &self.options
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoleSelectorError {
    pub option: &'static str,
    pub role: AriaRole,
}

impl Display for RoleSelectorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "role `{}` does not support `{}` filtering",
            self.role, self.option
        )
    }
}

impl Error for RoleSelectorError {}

fn validate_role_options(role: AriaRole, options: &RoleOptions) -> Result<(), RoleSelectorError> {
    if options.name.is_some() && options.name_regex.is_some() {
        return Err(RoleSelectorError {
            option: "name and name_regex together",
            role,
        });
    }
    if options.description.is_some() && options.description_regex.is_some() {
        return Err(RoleSelectorError {
            option: "description and description_regex together",
            role,
        });
    }
    let require = |present, supported: &[AriaRole], option| {
        if present && !supported.contains(&role) {
            Err(RoleSelectorError { option, role })
        } else {
            Ok(())
        }
    };
    require(
        options.checked.is_some(),
        &[
            AriaRole::Checkbox,
            AriaRole::MenuItemCheckbox,
            AriaRole::Option,
            AriaRole::Radio,
            AriaRole::Switch,
            AriaRole::MenuItemRadio,
            AriaRole::Treeitem,
        ],
        "checked",
    )?;
    require(options.pressed.is_some(), &[AriaRole::Button], "pressed")?;
    require(
        options.selected.is_some(),
        &[
            AriaRole::GridCell,
            AriaRole::Option,
            AriaRole::Row,
            AriaRole::Tab,
            AriaRole::RowHeader,
            AriaRole::ColumnHeader,
            AriaRole::Treeitem,
        ],
        "selected",
    )?;
    require(
        options.expanded.is_some(),
        &[
            AriaRole::Application,
            AriaRole::Button,
            AriaRole::Checkbox,
            AriaRole::Combobox,
            AriaRole::GridCell,
            AriaRole::Link,
            AriaRole::Listbox,
            AriaRole::MenuItem,
            AriaRole::Row,
            AriaRole::RowHeader,
            AriaRole::Tab,
            AriaRole::Treeitem,
            AriaRole::ColumnHeader,
            AriaRole::MenuItemCheckbox,
            AriaRole::MenuItemRadio,
            AriaRole::Switch,
        ],
        "expanded",
    )?;
    require(
        options.level.is_some(),
        &[
            AriaRole::Heading,
            AriaRole::ListItem,
            AriaRole::Row,
            AriaRole::Treeitem,
        ],
        "level",
    )
}

/// Backend-neutral, lazy selector expression.
///
/// The initial engine is CSS. Composition nodes operate on the resolved set
/// without materializing DOM identities in the client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selector {
    Css(CssSelector),
    Text(TextMatcher),
    LegacyText(TextMatcher),
    TextRegex(RegexMatcher),
    Label(TextMatcher),
    LabelRegex(RegexMatcher),
    Role(RoleSelector),
    RolePattern(RolePatternSelector),
    AttributeText {
        attribute: UserFacingAttribute,
        matcher: TextMatcher,
    },
    AttributeRegex {
        attribute: UserFacingAttribute,
        matcher: RegexMatcher,
    },
    TestIdText {
        attributes: TestIdAttributes,
        value: String,
    },
    TestIdRegex {
        attributes: TestIdAttributes,
        matcher: RegexMatcher,
    },
    Nth {
        base: Box<Self>,
        index: i32,
    },
    FilterText {
        base: Box<Self>,
        text: String,
        exact: bool,
    },
    FilterNotText {
        base: Box<Self>,
        text: String,
        exact: bool,
    },
    FilterTextRegex {
        base: Box<Self>,
        matcher: RegexMatcher,
    },
    FilterNotTextRegex {
        base: Box<Self>,
        matcher: RegexMatcher,
    },
    FilterHas {
        base: Box<Self>,
        descendant: Box<Self>,
    },
    FilterHasNot {
        base: Box<Self>,
        descendant: Box<Self>,
    },
    FilterVisible {
        base: Box<Self>,
        visible: bool,
    },
    And {
        left: Box<Self>,
        right: Box<Self>,
    },
    Or {
        left: Box<Self>,
        right: Box<Self>,
    },
    Within {
        base: Box<Self>,
        descendant: Box<Self>,
    },
}

impl Selector {
    pub fn css(value: impl Into<String>) -> Result<Self, CssSelectorError> {
        CssSelector::new(value).map(Self::Css)
    }

    pub fn text(value: impl Into<String>, exact: bool) -> Self {
        Self::Text(TextMatcher::new(value, exact))
    }

    pub fn legacy_text(value: impl Into<String>) -> Self {
        Self::LegacyText(TextMatcher::new(value, true))
    }

    pub fn text_regex(matcher: RegexMatcher) -> Self {
        Self::TextRegex(matcher)
    }

    pub fn label(value: impl Into<String>, exact: bool) -> Self {
        Self::Label(TextMatcher::new(value, exact))
    }

    pub fn label_regex(matcher: RegexMatcher) -> Self {
        Self::LabelRegex(matcher)
    }

    pub fn role(role: AriaRole, options: RoleOptions) -> Result<Self, RoleSelectorError> {
        if options.uses_regex() {
            RolePatternSelector::new(role, options).map(Self::RolePattern)
        } else {
            RoleSelector::new(role, options).map(Self::Role)
        }
    }

    pub fn attribute_text(
        attribute: UserFacingAttribute,
        value: impl Into<String>,
        exact: bool,
    ) -> Self {
        Self::AttributeText {
            attribute,
            matcher: TextMatcher::new(value, exact),
        }
    }

    pub fn attribute_regex(attribute: UserFacingAttribute, matcher: RegexMatcher) -> Self {
        Self::AttributeRegex { attribute, matcher }
    }

    pub fn test_id(value: impl Into<String>) -> Self {
        Self::test_id_with_attributes(TestIdAttributes::standard(), value)
    }

    pub fn test_id_with_attributes(attributes: TestIdAttributes, value: impl Into<String>) -> Self {
        Self::TestIdText {
            attributes,
            value: value.into(),
        }
    }

    pub fn test_id_regex(matcher: RegexMatcher) -> Self {
        Self::test_id_regex_with_attributes(TestIdAttributes::standard(), matcher)
    }

    pub fn test_id_regex_with_attributes(
        attributes: TestIdAttributes,
        matcher: RegexMatcher,
    ) -> Self {
        Self::TestIdRegex {
            attributes,
            matcher,
        }
    }

    pub fn nth(self, index: i32) -> Self {
        Self::Nth {
            base: Box::new(self),
            index,
        }
    }

    pub fn filter_text(self, text: impl Into<String>, exact: bool) -> Self {
        Self::FilterText {
            base: Box::new(self),
            text: text.into(),
            exact,
        }
    }

    pub fn filter_not_text(self, text: impl Into<String>, exact: bool) -> Self {
        Self::FilterNotText {
            base: Box::new(self),
            text: text.into(),
            exact,
        }
    }

    pub fn filter_text_regex(self, matcher: RegexMatcher) -> Self {
        Self::FilterTextRegex {
            base: Box::new(self),
            matcher,
        }
    }

    pub fn filter_not_text_regex(self, matcher: RegexMatcher) -> Self {
        Self::FilterNotTextRegex {
            base: Box::new(self),
            matcher,
        }
    }

    pub fn filter_has(self, descendant: Self) -> Self {
        Self::FilterHas {
            base: Box::new(self),
            descendant: Box::new(descendant),
        }
    }

    pub fn filter_has_not(self, descendant: Self) -> Self {
        Self::FilterHasNot {
            base: Box::new(self),
            descendant: Box::new(descendant),
        }
    }

    pub fn filter_visible(self, visible: bool) -> Self {
        Self::FilterVisible {
            base: Box::new(self),
            visible,
        }
    }

    pub fn and(self, other: Self) -> Self {
        Self::And {
            left: Box::new(self),
            right: Box::new(other),
        }
    }

    pub fn or(self, other: Self) -> Self {
        Self::Or {
            left: Box::new(self),
            right: Box::new(other),
        }
    }

    /// Queries `descendant` independently below every element matched by this
    /// selector, mirroring `locator.get_by_*` chaining rather than intersection.
    pub fn within(self, descendant: Self) -> Self {
        Self::Within {
            base: Box::new(self),
            descendant: Box::new(descendant),
        }
    }
}

impl From<CssSelector> for Selector {
    fn from(value: CssSelector) -> Self {
        Self::Css(value)
    }
}

impl Display for Selector {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Css(css) => formatter.write_str(css.as_str()),
            Self::Text(text) => write!(
                formatter,
                "get_by_text({:?}, exact={})",
                text.value(),
                text.exact()
            ),
            Self::LegacyText(text) => {
                write!(formatter, "legacy_text({:?})", text.value())
            }
            Self::TextRegex(regex) => write!(
                formatter,
                "get_by_text_regex({:?}, flags={:?})",
                regex.source(),
                regex.flags()
            ),
            Self::Label(text) => write!(
                formatter,
                "get_by_label({:?}, exact={})",
                text.value(),
                text.exact()
            ),
            Self::LabelRegex(regex) => write!(
                formatter,
                "get_by_label_regex({:?}, flags={:?})",
                regex.source(),
                regex.flags()
            ),
            Self::Role(role) => format_role_selector(formatter, role.role(), role.options()),
            Self::RolePattern(role) => format_role_selector(formatter, role.role(), role.options()),
            Self::AttributeText { attribute, matcher } => write!(
                formatter,
                "get_by_{}({:?}, exact={})",
                attribute.method_name(),
                matcher.value(),
                matcher.exact()
            ),
            Self::AttributeRegex { attribute, matcher } => write!(
                formatter,
                "get_by_{}_regex({:?}, flags={:?})",
                attribute.method_name(),
                matcher.source(),
                matcher.flags()
            ),
            Self::TestIdText { attributes, value } => write!(
                formatter,
                "get_by_test_id({value:?}, attributes={:?})",
                attributes.as_csv()
            ),
            Self::TestIdRegex {
                attributes,
                matcher,
            } => write!(
                formatter,
                "get_by_test_id_regex({:?}, flags={:?}, attributes={:?})",
                matcher.source(),
                matcher.flags(),
                attributes.as_csv()
            ),
            Self::Nth { base, index } => write!(formatter, "{base}.nth({index})"),
            Self::FilterText { base, text, exact } => {
                write!(formatter, "{base}.filter_text({text:?}, exact={exact})")
            }
            Self::FilterNotText { base, text, exact } => {
                write!(formatter, "{base}.filter_not_text({text:?}, exact={exact})")
            }
            Self::FilterTextRegex { base, matcher } => write!(
                formatter,
                "{base}.filter_text_regex({:?}, flags={:?})",
                matcher.source(),
                matcher.flags()
            ),
            Self::FilterNotTextRegex { base, matcher } => write!(
                formatter,
                "{base}.filter_not_text_regex({:?}, flags={:?})",
                matcher.source(),
                matcher.flags()
            ),
            Self::FilterHas { base, descendant } => {
                write!(formatter, "{base}.filter_has({descendant})")
            }
            Self::FilterHasNot { base, descendant } => {
                write!(formatter, "{base}.filter_has_not({descendant})")
            }
            Self::FilterVisible { base, visible } => {
                write!(formatter, "{base}.filter_visible({visible})")
            }
            Self::And { left, right } => write!(formatter, "({left}).and({right})"),
            Self::Or { left, right } => write!(formatter, "({left}).or({right})"),
            Self::Within { base, descendant } => {
                write!(formatter, "({base}).within({descendant})")
            }
        }
    }
}

fn format_role_selector(
    formatter: &mut Formatter<'_>,
    role: AriaRole,
    options: &RoleOptions,
) -> fmt::Result {
    write!(formatter, "get_by_role({:?}", role.to_string())?;
    if let Some(name) = &options.name {
        write!(formatter, ", name={:?}", name.value())?;
    } else if let Some(name) = &options.name_regex {
        write!(formatter, ", name=/{}/{}", name.source(), name.flags())?;
    }
    formatter.write_str(")")
}

#[cfg(test)]
mod tests {
    use super::{AriaRole, RegexMatcher, RegexMatcherError, RoleOptions, Selector};


    #[test]
    fn role_display_uses_playwright_call_log_shape() {
        // Arrange
        let selector = Selector::role(
            AriaRole::Button,
            RoleOptions::new().with_name("Hello Python", false),
        )
        .unwrap();

        // Act
        let display = selector.to_string();

        // Assert
        assert_eq!(display, "get_by_role(\"button\", name=\"Hello Python\")");
    }

    #[test]
    fn css_selector_rejects_an_empty_recipe_without_backend_work() {
        // Arrange
        let source = "  ";

        // Act
        let result = Selector::css(source);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn regex_flags_reject_noncanonical_order_in_the_value_model() {
        // Arrange
        let source = "button";

        // Act
        let result = RegexMatcher::new(source, "mi");

        // Assert
        assert_eq!(result, Err(RegexMatcherError::NonCanonicalOrder));
    }

    #[test]
    fn selector_composition_remains_lazy_and_backend_neutral() {
        // Arrange
        let root = Selector::css("#root").expect("CSS should be valid");
        let child = Selector::text("Save", true);

        // Act
        let selector = root.filter_visible(true).within(child);

        // Assert
        assert_eq!(
            selector.to_string(),
            "(#root.filter_visible(true)).within(get_by_text(\"Save\", exact=true))"
        );
    }
}
