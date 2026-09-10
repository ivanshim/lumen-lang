// A language definition read from JSON into the tables the assembler and
// the machine consult. Every tag must be present and of the right shape;
// keys beginning with `$` are notes and are skipped.

use std::collections::{HashMap, HashSet};

use serde_json::Value as Json;

use crate::value::Sort;
use crate::code::{Builtin, Action};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blocks {
    Indented,
    Braced,
    Worded,
}

#[derive(Clone, Debug)]
pub struct Brackets {
    pub open: String,
    pub close: String,
    pub between: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Operator {
    pub action: Action,
    pub level: u32,
    pub right_assoc: bool,
}

/// The kinds of complaint a language may have a word for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complaint {
    Warning,
    Notice,
    Deprecated,
    Fatal,
}

pub struct Lang {
    pub ident: String,
    pub extensions: Vec<String>,
    pub banner: String,

    pub with_as: Vec<String>,
    pub with_unready: Vec<String>,
    pub yield_from: Vec<String>,
    pub member_pipes: bool,
    pub tuple_unready: Vec<String>,
    pub bytes_unready: Vec<String>,
    pub format_unready: Vec<String>,
    pub identity_unready: Vec<String>,
    pub in_values: Vec<String>,
    pub in_not: Vec<String>,
    pub in_unready: Vec<String>,
    pub if_else: Vec<String>,
    pub assign_chain: bool,
    pub deferred_escapes: Vec<char>,
    pub line_comments: Vec<String>,
    pub block_comments: Vec<(String, String)>,
    pub quotes: Vec<char>,
    pub raw_quotes: Vec<char>,
    pub long_quotes: Vec<String>,
    pub raw_prefixes: Vec<char>,
    pub byte_prefixes: Vec<char>,
    pub plain_prefixes: Vec<char>,
    pub format_prefixes: Vec<char>,
    pub adjacent_strings: bool,
    pub string_amiss: Option<String>,
    pub byte_digits: Option<usize>,
    pub codepoint_digits: Option<usize>,
    pub wide_letter: Option<char>,
    pub wide_digits: Option<usize>,
    pub named_letter: Option<char>,
    pub escape_unavailable: Option<String>,
    pub escape_letters: Vec<char>,
    pub control_escapes: Vec<char>,
    pub continued_strings: bool,
    /// The letter that, after the escape mark, begins a character
    /// named by its number, and the brackets that number stands in.
    pub codepoint_letter: Option<char>,
    pub codepoint_open: Option<char>,
    pub codepoint_close: Option<char>,
    /// What the language says of a number badly written, and of one
    /// beyond the last character there is.
    pub codepoint_amiss: Option<String>,
    pub codepoint_beyond: Option<String>,
    /// The letter that, after the escape mark, begins a character named
    /// by its number in sixteens with no brackets about it: one digit
    /// or two, and the character of that number stands there.
    pub byte_letter: Option<char>,
    /// What a language says of a number it cannot read.
    pub number_amiss: Option<String>,
    pub prologue: Option<String>,
    pub point: Option<char>,
    pub base_mark: Option<char>,
    pub exponent_mark: Option<char>,
    pub hex_prefix: Option<String>,
    /// Each way of writing a number in a base of its own, with the base
    /// its digits are read in: `0x` for sixteen, `0b` for two.
    pub base_prefixes: Vec<(String, u32)>,
    /// A nought before more digits means base eight, as it does in the
    /// languages that grew from C.
    pub octal_lead: bool,
    /// Marks a program may put between the digits of a number to break
    /// them up, which count for nothing.
    pub digit_separators: Vec<char>,
    /// How many bits wide a whole number is, where a language says: a
    /// result that outgrows that width becomes a real instead.
    pub integer_bits: Option<usize>,
    /// How many bits wide a real is, where a language says its reals
    /// are binary numbers rather than exact ones, and how many
    /// significant digits one shows when simply written out.
    pub real_bits: Option<usize>,
    pub real_digits: Option<usize>,
    /// The names the run keeps its counts of figures under: how many a
    /// real written plainly carries, and how many one shown with its
    /// kind carries. The kernel makes each a cell of the run's own and
    /// asks it afresh every time it writes a real out, so a count the
    /// run writes there while it goes is the count that is followed.
    pub figures_binding: Option<String>,
    pub figures_shown_binding: Option<String>,
    pub unicode_names: bool,
    pub sigil: Option<char>,
    pub keywords_folded: bool,
    pub names_folded: bool,
    pub quote_for_names: Option<char>,
    pub symbols: Vec<String>,
    pub keywords: HashSet<String>,

    pub blocks: Blocks,
    /// Reverse Polish: words acting on one stack, no expressions.
    pub rpn: bool,
    pub indent_width: usize,
    pub block_opens: Vec<String>,
    pub block_closes: Vec<String>,
    pub block_intros: Vec<String>,
    pub stmt_ends: Vec<String>,
    pub stmt_separators: Vec<String>,
    pub grouping: Option<Brackets>,
    pub calling: Option<Brackets>,
    pub argument_labels: Vec<String>,
    pub array_brackets: Option<Brackets>,
    pub map_brackets: Option<Brackets>,
    /// `k => v` inside a literal, and between the two names of a foreach.
    pub pair_mark: Option<String>,
    pub index_brackets: Option<Brackets>,
    pub text_indexable: bool,
    pub slice_marks: Vec<String>,
    pub slice_ellipsis: Vec<String>,
    pub slice_zero: Option<String>,
    pub slice_bounds: Option<String>,
    pub slice_unsupported: Option<String>,
    pub slice_assign: Option<String>,
    pub slice_length: Vec<String>,
    pub slice_detached: Option<String>,

    pub true_words: Vec<String>,
    pub false_words: Vec<String>,
    pub null_words: Vec<String>,
    /// Nothing is shown as no text at all, rather than as the word a
    /// program writes for it: how PHP shows it.
    pub null_silent: bool,
    pub dyadic: HashMap<String, Operator>,
    pub monadic: HashMap<String, Operator>,
    pub pipe_words: Vec<String>,
    pub range_marks: Vec<String>,
    pub precedence: HashMap<String, u32>,

    pub assign_words: Vec<String>,
    pub let_words: Vec<String>,
    pub mutable_words: Vec<String>,
    pub type_marks: Vec<String>,
    /// An expression written as a kind is read and put by.
    pub annotation_marks: Vec<String>,
    pub annotation_amiss: Option<String>,
    pub annotation_target_unready: Option<String>,
    pub types_first: bool,
    pub if_words: Vec<String>,
    pub elif_words: Vec<String>,
    pub else_words: Vec<String>,
    pub while_words: Vec<String>,
    pub until_words: Vec<String>,
    pub for_words: Vec<String>,
    pub in_words: Vec<String>,
    pub return_words: Vec<String>,
    pub break_words: Vec<String>,
    pub continue_words: Vec<String>,
    pub function_words: Vec<String>,
    pub return_marks: Vec<String>,
    pub named_result: bool,
    pub pass_words: Vec<String>,

    pub dup_words: Vec<String>,
    pub drop_words: Vec<String>,
    pub swap_words: Vec<String>,
    pub over_words: Vec<String>,
    pub rot_words: Vec<String>,
    pub eval_words: Vec<String>,
    pub quote_open: Vec<String>,
    pub quote_close: Vec<String>,

    pub builtins: HashMap<String, Builtin>,
    pub print_sep: Vec<String>,
    pub print_end: Vec<String>,
    pub print_file: Vec<String>,
    pub print_flush: Vec<String>,
    pub print_file_error: Vec<String>,
    pub print_file_output: Vec<String>,
    pub print_file_unready: Vec<String>,
    pub print_sep_amiss: Vec<String>,
    pub print_end_amiss: Vec<String>,
    pub to_int_base: Vec<String>,
    pub to_int_base_amiss: Vec<String>,
    pub to_int_text_amiss: Vec<String>,
    pub to_int_text_required: Vec<String>,
    pub to_real_text: bool,
    pub to_real_text_amiss: Vec<String>,
    pub to_string_object: Vec<String>,
    pub to_string_encoding: Vec<String>,
    pub to_string_errors: Vec<String>,
    pub to_string_unready: Vec<String>,
    pub range_zero: Vec<String>,
    pub range_integer: Vec<String>,
    pub range_index: Vec<String>,
    pub holes: Vec<String>,
    pub args_binding: Option<String>,
    /// The same arguments as a list, the file the run was started with
    /// first, and how many there are in it.
    pub args_list: Option<String>,
    pub args_count: Option<String>,
    pub memo_binding: Option<String>,
    pub precision_binding: Option<String>,
    pub entry_binding: Option<String>,
    pub sort_bindings: Vec<(String, Sort)>,
    /// The shorter name each kind goes by where a complaint names one,
    /// in the order the kinds are listed under `system.kind.*`: whole,
    /// fraction, real, text, flag, array, nothing. A lone dash says the
    /// kind has no shorter name and the usual one stands.
    pub brief_kinds: Vec<String>,
    /// The word this language calls a thing's kind by, where it has
    /// one: a thing is of no kind the core knows.
    pub object_kind: Option<String>,
    /// The kind words that name no class of their own: what a value
    /// written with one of them may be is not settled by the word.
    pub loose_kinds: Vec<String>,
    /// Whether the kind of a value is given as text spelled by the
    /// `system.kind.*` names, rather than as a value of its own. Where
    /// it is, those names are not bound to anything.
    pub kind_spelled: bool,
    /// Whether a call may give more than the routine it calls names.
    /// A language that can read what a call was given lets it.
    pub spare_args: bool,
    /// Whether an assignment counts as an expression, its value what
    /// was written.
    pub assign_gives_value: bool,
    /// Whether every key of an array is either a whole number or text,
    /// so that a key spelling a whole number is that number.
    pub plain_keys: bool,
    /// The word a program uses for each kind of complaint, and the name
    /// it calls the line it is written on. A language with a word for a
    /// warning is one where reading a binding never written is a
    /// complaint rather than a stop.
    pub complaint_words: Vec<(Complaint, String)>,
    /// The name of the setting that asks for a complaint to be dressed
    /// for a reader of markup, and the dressing itself. Where the
    /// setting says no, the dressing is put by before the first word of
    /// the program is read and the plain words stand.
    pub markup_setting: Option<String>,
    /// What stands before the break of line a complaint opens with,
    /// what stands after that break and before the word naming the
    /// kind, and what stands after that word.
    pub markup_kind: Option<(String, String, String)>,
    /// What stands before and after the file a complaint names, and
    /// before and after the line, the kernel's own break of line coming
    /// after the last of it either way.
    pub markup_place: Option<(String, String)>,
    pub markup_line: Option<(String, String)>,
    /// What stands before the address of a word's page, between the
    /// address and the page's name, and after the name.
    pub markup_page: Option<(String, String, String)>,
    /// The name of the setting holding where the language's own pages
    /// are kept, the pieces standing before and after a word's name in
    /// the name of its page, and a mark of a word's name with what
    /// stands in its place in a page's.
    pub pages_setting: Option<String>,
    pub page_named: Option<(String, String)>,
    pub page_mark: Option<(String, String)>,
    pub line_binding: Option<String>,
    /// The names a program calls the routine it is written in, the
    /// class that routine belongs to, and the two written together.
    /// All three are known while the program is put together.
    pub routine_binding: Option<String>,
    pub class_binding: Option<String>,
    pub method_binding: Option<String>,
    /// The class a fault of the kernel's own is raised as, where a
    /// language names one, so that a program may take it like any other.
    pub fault_class: Option<String>,
    /// The class a fault over what was handed over to be walked is
    /// raised as.
    pub fault_walk: Option<String>,
    /// The class a fault of a kind is raised as, where a language names
    /// one: working with numbers, dividing by nought, and a value of a
    /// kind the work cannot take. Each stands in for the plain class
    /// where the language names it.
    pub fault_arithmetic: Option<String>,
    pub fault_division: Option<String>,
    pub fault_kind: Option<String>,
    /// The class of a fault about a value standing outside the range it
    /// may take.
    pub fault_value: Option<String>,
    /// The words a language puts before naming what an arithmetic step
    /// was handed, where one of them can take no part in it.
    pub operand_fault: Option<String>,
    /// The words for taking the remainder by nought, where a language
    /// tells that apart from dividing by nought, and the words for
    /// shifting bits by a number below nought. Where a language gives
    /// neither, the kernel's own words stand.
    pub fault_modulo: Option<String>,
    pub fault_shift: Option<String>,
    /// Whether the two shifts read each side for the number it is
    /// worth, the way arithmetic reads one, rather than reading it
    /// straight as bits.
    pub shift_by_number: bool,
    /// Whether dividing two whole numbers evenly gives a whole one.
    pub div_stays_whole: bool,
    /// The remainder is taken between whole numbers, whatever it is
    /// given: a real is brought to the whole number nearest nothing
    /// first, as a language whose remainder is a whole one does.
    pub mod_whole: bool,
    /// What a language says when a step onward or back is taken on text
    /// that spells no number. Naming either turns the rule on for that
    /// way: onward moves the last letter along, carrying; back leaves
    /// the text as it stands.
    pub step_up_text: Option<String>,
    pub step_down_text: Option<String>,
    /// Whether a binding never written is a complaint rather than a stop.
    pub warns_of_unwritten: bool,
    /// Whether the language says where a complaint happened, so that
    /// the assembler marks which line each statement is on.
    pub tells_place: bool,
    /// The names a program calls the file it is written in and the
    /// place that file lies in, where it has words for them.
    pub source_bindings: Vec<(String, String)>,
    /// Whether being equal is the looser question. A language with an
    /// operator for being the very same means something looser by being
    /// equal: text that spells a number stands for that number.
    pub loose_equality: bool,

    /// The ext.* labels: extensions of the core, read by the full
    /// kernels and ignored by the reference ones; absent means none.
    pub epilogue: Vec<String>,
    /// A second prologue, opening a run of code whose value is written
    /// out where it stands: PHP's `<?=`.
    pub prologue_echo: Option<String>,
    /// Whether the marker opening a run of code is found however it is
    /// written: PHP's `<?php` is `<?PHP` just as well.
    pub prologue_folded: bool,
    pub bare_calls: bool,
    /// Whether the writer is written as an operator and not as a call:
    /// brackets after it group what follows rather than holding its
    /// argument, so it takes the whole of the piece it stands before.
    pub writes_as_operator: bool,
    pub increments: Vec<String>,
    pub decrements: Vec<String>,
    pub interpolating: Vec<char>,
    /// The mark that opens a string written over lines, PHP's `<<<`.
    /// A label follows it, and the body runs to the line that label
    /// stands on again.
    pub heredoc: Option<String>,
    /// Whether a run of figures in eights after a backslash names a
    /// character by its number: the reference's `\101`.
    pub octal_escapes: bool,
    /// A shorter marker that opens a run of code, and the name of the
    /// setting that must be turned on before it does. Where the setting
    /// is off the marker is taken away before a word of the program is
    /// read, so that what stands after it is page and not program.
    pub prologue_brief: Option<String>,
    pub prologue_brief_setting: Option<String>,
    /// Whether text is held as the bytes it was written in rather than
    /// as the letters those bytes spell. Under this, every character of
    /// a piece of text stands for one byte and is worth its number, so
    /// that the length of a piece of text is the count of its bytes.
    pub text_is_bytes: bool,
    pub concat: Option<String>,
    pub foreach_words: Vec<String>,
    pub foreach_as_words: Vec<String>,
    pub c_for_words: Vec<String>,
    /// `x op= e` for every binary operator, when the switch is on.
    pub compound: HashMap<String, Action>,
    pub static_words: Vec<String>,
    pub tuple_marks: Vec<String>,
    pub class_bases_open: Vec<String>,
    pub class_bases_close: Vec<String>,
    pub class_unready: Vec<String>,
    pub del_words: Vec<String>,
    pub nonlocal_words: Vec<String>,
    pub nonlocal_unrun: Vec<String>,
    pub with_enter: Option<String>,
    pub with_leave: Option<String>,
    pub with_words: Vec<String>,
    pub with_as_words: Vec<String>,
    pub yield_words: Vec<String>,
    pub yield_from_words: Vec<String>,
    pub yield_unrun: Vec<String>,
    pub scope_unready: Vec<String>,
    pub global_words: Vec<String>,
    pub await_words: Vec<String>,
    pub async_words: Vec<String>,
    pub loop_else: bool,
    pub binding_unrun: String,
    pub del_unrun: String,
    pub import_values: bool,
    pub import_missing: Vec<String>,
    pub import_member_missing: Vec<String>,
    pub import_relative_unready: String,
    pub import_words: Vec<String>,
    pub import_from_words: Vec<String>,
    pub import_as_words: Vec<String>,
    pub math_floating: bool,
    pub module_cache: Vec<String>,
    pub module_names: Vec<String>,
    pub decorator_words: Vec<String>,
    pub decorator_amiss: Option<String>,
    pub const_words: Vec<String>,
    pub switch_words: Vec<String>,
    pub case_words: Vec<String>,
    pub default_words: Vec<String>,
    pub case_marks: Vec<String>,
    /// The words for closing a case with the mark that ends a statement
    /// rather than with the case's own, which a language may allow and
    /// still ask to be written the other way.
    pub case_mark_instead: Option<String>,
    /// The two signs of `test ? a : b`.
    pub ternary: Option<(String, String)>,
    /// A lone statement may stand where a block is expected.
    pub lone_stmt: bool,
    /// A top-level function is bound before anything else runs.
    pub hoisted: bool,
    /// Whether a routine declared anywhere is bound among the outermost
    /// bindings, so one written inside another is there for the whole
    /// run once the routine holding it has run.
    pub routines_outermost: bool,
    /// Whether a name merely read inside a routine is given a place of
    /// that routine's own, rather than meaning the outermost binding of
    /// that name. The place holds nothing until something writes there,
    /// and reading it while it does falls through to the outermost
    /// binding as before, so this tells only where a write from text
    /// read in while the run goes may land.
    pub own_names: bool,
    /// Whether a `static` written at the top of text read in while the
    /// run goes stands for a plain write of the name in the scope that
    /// read it, keeping nothing from one reading to the next.
    pub static_read_in: bool,
    /// The binding holding what the host found amiss in the request
    /// before the program ran, as a list of pieces of text, and the
    /// words for each thing that may be amiss. The host says only which
    /// of them it found; the words are the language's own.
    /// The binding holding the request's body as it came, so a program
    /// may read it for itself however the run reads it.
    pub body_binding: Option<String>,
    pub amiss_binding: Option<String>,
    pub amiss_words: Vec<(&'static str, String)>,
    /// Letters that open a decimal exponent in a number (1e9).
    pub exponent_letters: Vec<char>,
    /// A sign that leaves its operand as it is.
    pub plus_words: Vec<String>,
    pub if_else_words: Vec<String>,
    pub lambda_words: Vec<String>,
    pub lambda_unsupported: Option<String>,
    pub lambda_enclosing: Option<String>,
    pub identity_not: Vec<String>,
    pub identity_unsupported: Option<String>,
    pub membership_words: Vec<String>,
    pub membership_not: Vec<String>,
    pub membership_unsupported: Option<String>,
    pub chained_comparisons: bool,
    pub expression_assign: Vec<String>,
    pub ellipsis_words: Vec<String>,
    pub rem_formats_text: bool,
    pub format_unsupported: Option<String>,
    pub format_arguments: Option<String>,

    pub hush_words: Vec<String>,
    /// The mark written before a value to say that the value spells a
    /// name, and the name is what is meant.
    pub naming_words: Vec<String>,
    /// Whether a kind's name written within the grouping marks, before
    /// a value, makes the value that kind.
    pub casts_kinds: bool,
    /// Whether a value may stand where a member's name stands, so that
    /// the member is the one the value spells.
    pub members_by_value: bool,
    /// Whether a piece of text is a row of places, each holding one
    /// letter: a place may be written to as well as read, and one named
    /// by text is the number that text opens with. Text that may be read
    /// letter by letter is not always text with places in this sense.
    pub text_places: bool,
    /// What a language says when more than one letter is handed to a
    /// place in text, only the first of them going in. Nothing where a
    /// language holds its peace about it.
    pub text_place_first: Option<String>,
    /// What a language says of a name given to its binder of constants
    /// that spells one of a class's own values rather than a constant.
    /// Nothing where a language takes such a name as it comes.
    pub define_scoped: Option<String>,
    /// The words standing for all the outermost bindings taken as an
    /// array, so that a place in it is the binding whose name the place
    /// spells: how a language reaches a global from inside a routine.
    pub globals_words: Vec<String>,
    /// What a language has to say when a program asks to share a cell
    /// from something that has none: once where the asking is a write,
    /// and once where it is a routine giving back what it answers with.
    /// Nothing where a language holds its peace.
    pub unshared_written: Vec<String>,
    pub unshared_given: Vec<String>,
    /// And once where a call hands an argument to a parameter that takes
    /// a cell.
    pub unshared_handed: Vec<String>,
    /// A mark that opens a block where a bracket would, and the words
    /// that close one so opened: `if (c): ... endif;`. The statements
    /// run to whichever closing word comes next, and a word that ends
    /// the whole shape is taken with the block; one that opens another
    /// arm of it is left standing.
    pub instead_mark: Option<String>,
    pub instead_closes: Vec<String>,
    /// The words opening a body that runs before its test is asked, the
    /// test standing after it: `do { … } while (c);`.
    pub do_words: Vec<String>,
    /// Whether a piece of text spelling the name of a routine or a class
    /// may stand where the routine or the class itself would: `$f()`,
    /// `new $c`, `$c::m()`. The name is one of the outermost bindings,
    /// those being the only ones still there to be looked up as the run
    /// goes.
    pub spelled_stands: bool,
    /// Whether a class is known by its name however the name is written.
    pub classes_folded: bool,
    /// Whether a statement ends only where the terminator is written,
    /// a line end being no more than space.
    pub terminator_only: bool,
    /// The word a language puts before a program it cannot read.
    pub reading_word: Option<String>,
    /// The words a language puts before whatever stopped the reading.
    pub reading_unexpected: Option<String>,
    /// The words before a character named by its number, which is how a
    /// character that cannot be shown is best named.
    pub reading_character: Option<String>,
    /// The words on either side of a bracket left open, the line it was
    /// opened on where that is not the line the reading stopped on, the
    /// bracket that answered it wrongly, and a bracket answering none.
    /// A language that gives none of these is not weighed for balance
    /// at all: a bracket amiss shows up later as whatever the reading
    /// makes of it.
    pub unclosed_words: Option<(String, String)>,
    pub unclosed_line: Option<String>,
    pub mismatch_words: Option<(String, String)>,
    pub unmatched_words: Option<(String, String)>,
    /// The class a program that cannot be read is raised under.
    pub fault_reading: Option<String>,
    /// The words on either side of the line the reading of text was
    /// asked for on, which together name where that text stands.
    pub eval_place: Option<(String, String)>,
    /// The words for reading a file in that will not go on without it:
    /// where the file is not there the run is stopped instead of
    /// answering false. The words on either side of the file's name say
    /// so; a language naming none reads every file in the same way.
    pub include_demanded: Vec<String>,
    pub include_demanded_missing: Option<(String, String)>,
    /// What a language says of a key between the brackets of a name
    /// woven into text where the shorter writing does not take it.
    pub woven_index_words: Option<String>,
    /// Whether a flag becomes text as the number it stands for.
    pub flags_count: bool,
    /// What a language says when a builtin that reads what the running
    /// call was handed is reached where no call is running, one for each
    /// of the three; and what it says of a place below the first or past
    /// the last. Nothing where the kernel's own words will do.
    pub args_outside_all: Option<String>,
    pub args_outside_count: Option<String>,
    pub args_outside_at: Option<String>,
    pub args_below: Option<String>,
    pub args_beyond: Option<String>,
    /// The words that open a taking-apart: a list of places written on
    /// the left of a write, each taking the matching place of the value.
    pub unpack_words: Vec<String>,
    pub unpack_rest: Vec<String>,
    pub unpack_short: Option<String>,
    pub unpack_long: Option<String>,
    pub unpack_unwalkable: Option<String>,
    pub unpack_amiss: Option<String>,
    /// Whether writing into a place makes what is needed to hold it:
    /// an array where a name holds nothing, and one at each place along
    /// the way that is not there yet.
    pub makes_places: bool,
    /// Pieces of text the language counts as untrue besides text with
    /// nothing in it, and whether an array holding nothing is untrue.
    pub untrue_text: Vec<String>,
    pub untrue_empty: bool,
    /// `break n` and `continue n` leave n loops.
    pub break_levels: bool,
    /// The source is text with code between the prologue and the
    /// epilogue; what is outside them is written out as it stands.
    pub template: bool,
    /// `a[] = v` appends.
    pub append_index: bool,
    /// `for v in a` walks what a holds when a is not a range.
    pub for_collections: bool,
    pub comprehension_async: Vec<String>,
    pub comprehension_async_unavailable: Vec<String>,
    pub comprehension_target_unavailable: Vec<String>,
    pub sum_non_number: Vec<String>,
    pub range_non_integer: Vec<String>,
    pub range_zero_step: Vec<String>,

    pub comprehension_for: Vec<String>,
    pub comprehension_in: Vec<String>,
    pub comprehension_if: Vec<String>,
    pub set_literals: bool,
    pub array_spread: Vec<String>,
    pub map_spread: Vec<String>,
    pub collection_unwalkable: Vec<String>,
    pub spread_unmapped: Vec<String>,
    pub comprehension_unpack_amiss: Vec<String>,
    pub range_value: bool,


    /// Classes and their objects.
    pub class_words: Vec<String>,
    pub bases_open: Option<String>,
    pub bases_close: Option<String>,
    pub explicit_this: bool,
    pub extends_words: Vec<String>,
    pub new_words: Vec<String>,
    /// The name a method knows its own object by.
    pub this_word: Option<String>,
    /// The method run when an object is made.
    pub constructor: Option<String>,
    /// The method run when an object is let go, at the latest when the
    /// run ends.
    pub destructor: Option<String>,
    /// The method that answers for a property the thing does not hold,
    /// and the one that takes a write of such a property. Each is given
    /// the name that was asked for, the writer the value as well.
    pub reader: Option<String>,
    pub writer: Option<String>,
    /// The method a class answers a call it does not have with, given
    /// the name called and the arguments as an array.
    pub caller: Option<String>,
    /// A thing may be its own walk. The class of method names saying so,
    /// and the five methods of the walk: wind back, whether there is
    /// more, what stands here, what it is called, and step on.
    pub walker_class: Option<String>,
    pub walk_rewind: Option<String>,
    pub walk_more: Option<String>,
    pub walk_this: Option<String>,
    pub walk_key: Option<String>,
    pub walk_onward: Option<String>,
    /// A thing may instead hand over another to be walked in its stead:
    /// the class of method names saying so, and the method that hands
    /// the walk over.
    pub giver_class: Option<String>,
    pub walk_giver: Option<String>,
    /// What is said of a thing that hands over, to be walked in its
    /// stead, something that is no walk at all: the words before the
    /// name of what handed it over, and the words after.
    pub giver_unwalkable: Option<(String, String)>,
    /// The words for asking a thing that is its own walk to hand out
    /// the items' own cells, which it has none of.
    pub walk_no_cell: Option<String>,
    /// The words for marking the name a walk gives its keys to as taking
    /// a cell: a key is not a place and has no cell to hand out.
    pub walk_key_no_cell: Option<String>,
    /// Whether a walk that hands out the items' own cells keeps its
    /// place by the item it handed out rather than by counting: where
    /// the body moves that item along the array, or takes it out, the
    /// walk moves with it.
    pub walk_alive: bool,
    /// Words that may stand before a member and say nothing this kernel reads.
    pub modifier_words: Vec<String>,
    /// The modifiers saying how far a member may be reached from: only
    /// from the class that declares it, or from that class and those
    /// standing on it. A member with neither is open to all.
    pub hidden_words: Vec<String>,
    pub guarded_words: Vec<String>,
    /// The modifier marking a member the class keeps for itself.
    pub shared_words: Vec<String>,
    /// `object->member`, and `class::member`.
    pub member_mark: Option<String>,
    pub scope_mark: Option<String>,
    pub instanceof_words: Vec<String>,
    /// A class of method names only, and the word saying a class
    /// answers to one.
    /// `a ?? b`: a when it is something, else b.
    pub otherwise_mark: Option<String>,
    /// A bag of members a class may take in as its own, the word that
    /// takes it in, and the word that gives one of its members another
    /// name in the class taking it.
    pub trait_words: Vec<String>,
    pub uses_words: Vec<String>,
    /// The word a routine written where a value stands says, before the
    /// names it carries away from around it.
    pub carries_words: Vec<String>,
    pub bind_names: bool,
    pub carries_pairs: Vec<String>,
    pub keyword_only: Vec<String>,
    pub positional_only: Vec<String>,
    pub call_spread: Vec<String>,
    pub call_spread_pairs: Vec<String>,
    pub call_amiss: Vec<String>,
    pub call_missing: Vec<String>,
    pub call_unknown: Vec<String>,
    pub call_duplicate: Vec<String>,
    pub call_builtin_amiss: Vec<String>,
    pub spread_amiss: Vec<String>,
    pub spread_pairs_amiss: Vec<String>,
    pub defaults_amiss: Vec<String>,
    pub parameters_amiss: Vec<String>,

    /// The word a routine written short says, and the mark standing
    /// between its parameters and the one expression it is: `fn ($x) =>
    /// $x + 1`. Such a routine takes with it every name around it.
    pub short_function: Option<(String, String)>,
    pub uses_alias: Vec<String>,
    pub interface_words: Vec<String>,
    pub implements_words: Vec<String>,
    pub parent_words: Vec<String>,
    pub self_words: Vec<String>,
    /// Signs a name may be led by, which say nothing: PHP's `\Error`.
    pub name_leads: Vec<char>,
    pub assert_words: Vec<String>,
    pub assert_kind: Option<String>,
    pub catch_as: Vec<String>,
    pub catch_invalid: Option<String>,
    pub catch_tuple_open: Option<String>,
    pub catch_tuple_close: Option<String>,
    pub catch_group: Option<String>,
    pub catch_group_unsupported: Option<String>,
    pub try_else: bool,
    pub throw_from: Vec<String>,
    pub throw_empty: Option<String>,
    pub try_words: Vec<String>,
    pub catch_words: Vec<String>,
    pub finally_words: Vec<String>,
    pub throw_words: Vec<String>,
    /// Between the classes one catch takes.
    pub catch_between: Option<String>,
    /// The sign that makes one name stand for another's cell.
    pub reference_mark: Option<String>,
    /// Reading a place an array does not hold gives nothing, rather
    /// than stopping the program.
    pub absent_index: bool,
    /// The words for using a value that is neither an array nor
    /// anything with places as though it had them.
    pub scalar_index: Option<String>,
    /// The words for naming a place by nothing at all, which a language
    /// may take as naming the place named by the empty text.
    pub nothing_index: Option<String>,
    /// What the language calls the parts of a web request: the name for
    /// each group the host gathers, and one for all of them together.
    pub request_bindings: Vec<(String, String)>,
}

/// Every tag with its shape: w a word list, s a string, b a switch,
/// n a size or null, o a string or null, t precedence tiers, x a word
/// list this kernel gives no meaning and passes over, as it passes over
/// an ext.* label it does not read.
const LABELS: &str = "
n format_version | s language | w extensions | w lexical.comment_line
w lexical.comment_block.open | w lexical.comment_block.close | w lexical.string_quotes | w lexical.raw_quotes
w lexical.string_escapes | w lexical.prologue | w lexical.name_quote | w lexical.number.decimal_point
w lexical.number.base_marker | w lexical.number.exponent_marker | w lexical.number.hex_prefix | b lexical.keywords_case_insensitive
b identifier.unicode | w identifier.variable_prefix | b identifier.case_insensitive | s block.style
w block.open | w block.close | w block.intro | n block.indent_size
w stmt.terminator | s syntax.notation | w syntax.group.open | w syntax.group.close
w syntax.call.open | w syntax.call.separator | w syntax.call.close | w syntax.call.label
w syntax.array.open | w syntax.array.separator | w syntax.array.close | w syntax.map.open
w syntax.map.separator | w syntax.map.pair | w syntax.map.close | w literal.true
w literal.false | w literal.null | b literal.null.silent | t op.precedence | w op.right_associative
w op.add | w op.sub | w op.mul | w op.div
o op.div.result | b op.mod.whole | w op.quot | w op.rem | w op.pow
w op.eq | w op.ne | w op.lt | w op.le
w op.gt | w op.ge | w op.and | w op.or
w op.not | w op.negate | w op.concat | w op.range
w op.index.open | w op.index.close | b op.index.strings | w op.pipe
w stmt.assign | w stmt.let | w stmt.let.mutable | w stmt.let.annotation
b stmt.let.type_first | w stmt.if | w stmt.elif | w stmt.else
w stmt.while | w stmt.until | w stmt.for | w stmt.for.in
w stmt.foreach | w stmt.foreach.as | w stmt.foreach.pair | w stmt.return
w stmt.break | w stmt.continue | w stmt.function | w stmt.function.returns
b stmt.function.result_by_name | w stmt.pass | x stmt.emit | w stack.dup
w stack.drop | w stack.swap | w stack.over | w stack.rot
w stack.eval | w stack.program.open | w stack.program.close | w builtin.emit
w builtin.print | w builtin.write | w builtin.print.placeholder | w builtin.len
w builtin.char_at | w builtin.ord | w builtin.chr | w builtin.typeof
w builtin.error | w builtin.extern | w builtin.range | w builtin.real
w builtin.num | w builtin.den | w builtin.push | w builtin.get
w builtin.put | w builtin.precision | w builtin.to_string | w builtin.to_int
w builtin.to_real | w system.args | w system.memoization | w system.real_default_precision
w system.entry | w system.kind.integer | w system.kind.rational | w system.kind.real
w system.kind.string | w system.kind.boolean | w system.kind.array | w system.kind.null
b system.flag.counts
";

/// The extension labels a definition may add beyond the core; a
/// missing one reads as empty (or off).
const EXT_LABELS: &str = "
w ext.lexical.string.long | w ext.op.lambda | w ext.op.tuple | w ext.stmt.class.bases.open | w ext.stmt.class.bases.close | w ext.stmt.class.unready | w ext.stmt.del | w ext.stmt.nonlocal | w ext.stmt.nonlocal.unrun | w ext.stmt.with | w ext.stmt.with.as | w ext.stmt.yield | w ext.stmt.yield.from | w ext.stmt.yield.unrun | w ext.system.scope.unready

w ext.op.index.slice.ellipsis | w ext.op.index.slice | w ext.op.index.slice.zero | w ext.op.index.slice.bounds | w ext.op.index.slice.unsupported | w ext.op.index.slice.assign | w ext.op.index.slice.length | w ext.op.index.slice.detached
w ext.op.comprehension.async | w ext.op.comprehension.async.unavailable | w ext.op.comprehension.target.unavailable | w ext.builtin.sum.non_number | w ext.builtin.range.non_integer | w ext.builtin.range.zero_step

w ext.op.comprehension.for | w ext.op.comprehension.in | w ext.op.comprehension.if | b ext.syntax.set | w ext.syntax.array.spread | w ext.syntax.map.spread | w ext.syntax.collection.unwalkable | w ext.syntax.map.spread.unmapped | w ext.op.comprehension.unpack.amiss | b ext.builtin.range.value | w ext.builtin.sum | w ext.builtin.list | w ext.builtin.any

w ext.lexical.epilogue | w ext.system.args.list | w ext.system.args.count | w ext.lexical.prologue.echo | b ext.lexical.prologue.folded | w ext.builtin.echo | b ext.syntax.call.bare | w ext.op.increment
w ext.op.decrement | w ext.lexical.interpolating_quotes | w ext.lexical.heredoc | b ext.lexical.escape.octal | b ext.system.text.bytes | w ext.lexical.prologue.brief | w ext.lexical.prologue.brief.setting | w ext.stmt.for.c | b ext.op.assign.compound
 | w ext.stmt.del.unrun | w ext.stmt.binding.unrun | b ext.stmt.loop.else | w ext.stmt.async | w ext.op.await | w ext.stmt.static | w ext.stmt.global | w ext.stmt.decorator | w ext.stmt.decorator.amiss | w ext.stmt.const | w ext.builtin.define | w ext.builtin.define.class_constant
b ext.stmt.import.value | w ext.stmt.import.missing | w ext.stmt.import.member.missing | w ext.stmt.import.relative.unready
w ext.builtin.program.namespace
w ext.builtin.member.get
w ext.builtin.member.set
w ext.builtin.instance
w ext.builtin.module.load
w ext.builtin.copy
w ext.stmt.with.enter | w ext.stmt.with.leave
w ext.system.module.cache
b ext.builtin.math.floating
w ext.builtin.call.outcome
w ext.stmt.import | w ext.stmt.import.from | w ext.stmt.import.as | w ext.system.module.name

w ext.builtin.var_dump | w ext.stmt.switch | w ext.stmt.case | w ext.stmt.default
w ext.stmt.case.mark | w ext.stmt.case.mark.instead | w ext.op.ternary | b ext.block.lone_statement | b ext.stmt.function.hoisted | b ext.stmt.function.outermost
w ext.system.request.amiss | w ext.system.request.amiss.boundary | w ext.system.request.amiss.boundary.wrong | w ext.system.request.amiss.part | w ext.system.request.amiss.body.large | w ext.system.request.body
w ext.op.if_else | w ext.op.lambda.unsupported | w ext.op.lambda.enclosing | w ext.op.identical.negated | w ext.op.identical.unsupported | w ext.op.in | w ext.op.in.negated | w ext.op.in.unsupported | b ext.op.compare.chained | w ext.op.assign.expression | w ext.literal.ellipsis | b ext.op.rem.formats_text | w ext.op.rem.format.unsupported | w ext.op.rem.format.arguments
w ext.lexical.number.exponent | w ext.op.plus | b ext.stmt.break.levels
w ext.builtin.array | b ext.op.index.append | b ext.stmt.for.collection | w ext.builtin.print_r
w ext.stmt.terminator | w ext.stmt.annotation | w ext.stmt.annotation.amiss | w ext.stmt.annotation.target.unready | w ext.stmt.function.returns | w ext.stmt.class | w ext.stmt.class.extends | w ext.stmt.class.new
w ext.stmt.class.this | w ext.stmt.class.constructor | w ext.stmt.class.destructor | w ext.stmt.class.reader | w ext.stmt.class.writer | w ext.stmt.class.caller
w ext.op.walk.class | w ext.op.walk.rewind | w ext.op.walk.more | w ext.op.walk.this | w ext.op.walk.key
w ext.op.walk.onward | w ext.op.walk.giver.class | w ext.op.walk.giver | w ext.op.walk.no_cell | w ext.op.walk.key.no_cell | b ext.op.walk.live | w ext.builtin.array.front | w ext.stmt.class.modifier | w ext.stmt.class.hidden | w ext.stmt.class.guarded | w ext.stmt.class.shared
w ext.op.member | w ext.op.scope | w ext.op.instanceof | w ext.stmt.class.parent
w ext.stmt.class.self | w ext.lexical.name_lead | w ext.stmt.assert | w ext.stmt.assert.kind | w ext.stmt.catch.invalid | w ext.stmt.catch.as | w ext.stmt.catch.tuple.open | w ext.stmt.catch.tuple.close | w ext.stmt.catch.group | w ext.stmt.catch.group.unsupported | b ext.stmt.try.else | w ext.stmt.throw.from | w ext.stmt.throw.empty | w ext.stmt.try | w ext.stmt.catch
w ext.stmt.finally | w ext.stmt.throw | w ext.stmt.catch.separator | w ext.op.reference
w ext.system.request.query | w ext.system.request.form | w ext.system.request.cookies | w ext.system.request.server
w ext.system.request.env | w ext.system.request.files | w ext.system.request.all | w ext.system.request.settings | b ext.op.index.absent | w ext.op.index.scalar | w ext.op.index.nothing | w ext.stmt.class.interface | w ext.stmt.class.implements | w ext.op.compare | w ext.builtin.unset | b ext.lexical.template | w ext.op.otherwise
w ext.op.bit.and | w ext.op.bit.or | w ext.op.bit.xor | w ext.op.bit.not | w ext.op.bit.left | w ext.op.bit.right | b ext.op.bit.shift.numbers
w ext.op.identical | w ext.op.not_identical | b ext.system.kind.spelled
w ext.builtin.args.all | w ext.builtin.args.count | w ext.builtin.args.at
w ext.builtin.args.all.outside | w ext.builtin.args.count.outside | w ext.builtin.args.at.outside
w ext.builtin.args.at.below | w ext.builtin.args.at.beyond | b ext.op.assign.value | b ext.op.index.plain_keys
w ext.system.source.file | w ext.system.source.directory | w ext.system.source.line | w ext.system.runner
w ext.system.complaint.warning | w ext.system.complaint.notice | w ext.system.complaint.deprecated | w ext.system.complaint.fatal | w ext.system.complaint.reading
w ext.system.complaint.markup.setting | w ext.system.complaint.markup.kind | w ext.system.complaint.markup.place | w ext.system.complaint.markup.line | w ext.system.complaint.markup.reference
w ext.system.complaint.reference.setting | w ext.system.complaint.reference.page | w ext.system.complaint.reference.mark
w ext.builtin.include.demanded | w ext.builtin.include.demanded.missing
w ext.system.fault.class | w ext.builtin.time_limit | w ext.system.kind.brief
w ext.builtin.file.read | w ext.builtin.file.write | w ext.builtin.file.exists | w ext.builtin.file.remove | w ext.builtin.shell | w ext.builtin.wait | w ext.builtin.net.ask | w ext.builtin.run.begin | w ext.builtin.run.end
w ext.builtin.room.used | w ext.builtin.room.most | w ext.builtin.room.most.forget | w ext.builtin.room.limit
w ext.builtin.eval | w ext.builtin.include | w ext.builtin.include.once
w ext.builtin.output.hold | w ext.builtin.output.held | w ext.builtin.output.drop | w ext.builtin.output.depth | w ext.builtin.output.begun | w ext.builtin.at_end | w ext.builtin.complaint.handler | w ext.builtin.complaint.say | w ext.op.hush | w ext.builtin.isset | w ext.builtin.empty | w ext.stmt.do | b ext.op.index.makes | w ext.builtin.calls | w ext.system.kind.object | w ext.builtin.uncaught | w ext.builtin.classes | w ext.builtin.routines | w ext.builtin.spelled | w ext.builtin.class.beneath | w ext.builtin.math | w ext.builtin.class.methods | w ext.builtin.class.properties | b ext.builtin.write.operator | w ext.system.kind.loose | w ext.builtin.clock | w ext.stmt.class.trait | w ext.stmt.class.uses | w ext.stmt.class.uses.alias | b ext.syntax.call.bind_names | w ext.stmt.function.carries.pairs | w ext.stmt.function.keyword_only | w ext.stmt.function.positional_only | w ext.syntax.call.spread | w ext.syntax.call.spread.pairs | w ext.syntax.call.amiss | w ext.syntax.call.amiss.missing | w ext.syntax.call.amiss.unknown | w ext.syntax.call.amiss.duplicate | w ext.syntax.call.amiss.builtin | w ext.builtin.print.sep | w ext.builtin.print.end | w ext.builtin.print.file | w ext.builtin.print.flush | w ext.builtin.print.file.error | w ext.builtin.print.file.output | w ext.builtin.print.file.unready | w ext.builtin.print.sep.amiss | w ext.builtin.print.end.amiss | w ext.builtin.to_int.base | w ext.builtin.to_int.base.amiss | w ext.builtin.to_int.text.amiss | w ext.builtin.to_int.text.required | b ext.builtin.to_real.text | w ext.builtin.to_real.text.amiss | w ext.builtin.to_string.object | w ext.builtin.to_string.encoding | w ext.builtin.to_string.errors | w ext.builtin.to_string.unready | b ext.builtin.range.value | w ext.builtin.range.zero | w ext.builtin.range.integer | w ext.builtin.range.index | w ext.syntax.call.spread.amiss | w ext.syntax.call.spread.pairs.amiss | w ext.stmt.function.defaults.amiss | w ext.stmt.function.parameters.amiss | w ext.stmt.function.carries | w ext.stmt.function.short
w ext.system.untrue.text | b ext.system.untrue.empty_array | w ext.builtin.exit
w ext.system.fault.operands | w ext.op.increment.text | w ext.op.decrement.text
w ext.system.fault.class.arithmetic | w ext.system.fault.class.division | w ext.system.fault.class.kind | w ext.system.fault.class.value | w ext.system.fault.class.walk | w ext.op.walk.giver.unwalkable
w ext.system.fault.modulo | w ext.system.fault.shift
w ext.op.name_by_value | b ext.op.cast | w ext.stmt.unpack
w ext.op.tuple | w ext.stmt.unpack.rest | b ext.stmt.assign.chain
w ext.stmt.unpack.short | w ext.stmt.unpack.long | w ext.stmt.unpack.unwalkable | w ext.stmt.unpack.amiss
w ext.system.source.routine | w ext.system.source.class | w ext.system.source.method
b ext.op.member.by_value | b ext.op.index.text | w ext.op.index.text.first | w ext.system.globals
w ext.op.reference.unshared.written | w ext.op.reference.unshared.given | w ext.op.reference.unshared.handed
b ext.stmt.terminator.only | w ext.stmt.separator
w ext.stmt.block.instead | w ext.stmt.block.instead.close | b ext.op.spelled | b ext.system.class.folded

 | w ext.lexical.string.prefix.raw | w ext.lexical.string.prefix.bytes | w ext.lexical.string.prefix.plain | w ext.lexical.string.prefix.format | b ext.lexical.string.adjacent | w ext.lexical.string.amiss | n ext.lexical.escape.byte.digits | n ext.lexical.escape.codepoint.digits | w ext.lexical.escape.codepoint.wide | n ext.lexical.escape.codepoint.wide.digits | w ext.lexical.escape.named | w ext.lexical.escape.unavailable
b ext.lexical.escape.continued | w ext.lexical.escape.controls | w ext.lexical.escape.codepoint | w ext.lexical.escape.codepoint.open | w ext.lexical.escape.codepoint.close
w ext.lexical.escape.codepoint.amiss | w ext.lexical.escape.codepoint.beyond | w ext.lexical.number.amiss
w ext.lexical.escape.byte | w ext.lexical.interpolating.index.amiss | w ext.builtin.eval.place
w ext.system.reading.unexpected | w ext.system.reading.unexpected.character | w ext.system.fault.class.reading
w ext.system.reading.unclosed | w ext.system.reading.unclosed.line | w ext.system.reading.unclosed.mismatch | w ext.system.reading.unmatched
w ext.lexical.number.binary_prefix | w ext.lexical.number.octal_prefix | b ext.lexical.number.octal_lead | w ext.lexical.number.separator
n ext.system.integer.bits | n ext.system.real.bits | n ext.system.real.digits
w ext.system.real.figures | w ext.system.real.figures.shown
w ext.stmt.class.bases.open | w ext.stmt.class.bases.close | b ext.stmt.class.this.explicit
b ext.op.member.pipes | w ext.stmt.class.unready
b ext.stmt.function.own_names | b ext.stmt.static.read_in

w ext.stmt.with.unready
b ext.op.member.pipes
w ext.op.tuple.unready
w ext.lexical.string.prefix.bytes.unready
w ext.lexical.string.prefix.format.unready
b ext.stmt.assign.chain
w ext.lexical.escape.deferred
";

fn shapes_of(table: &'static str) -> Vec<(char, &'static str)> {
    table
        .split(['\n', '|'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|entry| (entry.chars().next().unwrap(), entry[2..].trim()))
        .collect()
}

fn number_shapes() -> Vec<(char, &'static str)> {
    shapes_of(LABELS)
}

static ABSENT_LIST: Json = Json::Array(Vec::new());
static ABSENT_SWITCH: Json = Json::Bool(false);
/// A count no definition gives stands for nothing at all.
static ABSENT_COUNT: Json = Json::Null;

struct Reader<'a>(&'a serde_json::Map<String, Json>);

impl<'a> Reader<'a> {
    fn field(&self, key: &str) -> Result<&'a Json, String> {
        if let Some(value) = self.0.get(key) {
            return Ok(value);
        }
        match shapes_of(EXT_LABELS).iter().find(|(_, tag)| *tag == key) {
            Some(('b', _)) => Ok(&ABSENT_SWITCH),
            Some(('n', _)) => Ok(&ABSENT_COUNT),
            Some(_) => Ok(&ABSENT_LIST),
            None => Err(format!("missing label '{key}'")),
        }
    }

    fn strings(&self, key: &str) -> Result<Vec<String>, String> {
        let Json::Array(items) = self.field(key)? else {
            return Err(format!("label '{key}' must be a list of non-empty strings"));
        };
        items
            .iter()
            .map(|item| match item {
                Json::String(s) if !s.is_empty() => Ok(s.clone()),
                _ => Err(format!("label '{key}' must be a list of non-empty strings")),
            })
            .collect()
    }

    fn head(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.strings(key)?.into_iter().next())
    }

    /// A label whose words stand on either side of what the kernel puts
    /// between them: the first word before it, the second after. One
    /// word alone leaves the kernel nothing to close with, so it is
    /// refused rather than half read.
    fn around(&self, key: &str) -> Result<Option<(String, String)>, String> {
        let mut said = self.strings(key)?.into_iter();
        match (said.next(), said.next()) {
            (Some(before), Some(after)) => Ok(Some((before, after))),
            (Some(_), None) => Err(format!("label '{key}' wants a word on either side of what it names")),
            _ => Ok(None),
        }
    }

    /// A label whose words stand about two things the kernel puts
    /// between them: a word before the first, a word between the two,
    /// and a word after the second. Fewer than three leaves the kernel
    /// short of a piece, so they are refused rather than half read.
    fn about_two(&self, key: &str) -> Result<Option<(String, String, String)>, String> {
        let mut said = self.strings(key)?.into_iter();
        match (said.next(), said.next(), said.next()) {
            (Some(first), Some(second), Some(third)) => Ok(Some((first, second, third))),
            (Some(_), ..) => Err(format!("label '{key}' wants three words about the two things it names")),
            _ => Ok(None),
        }
    }

    fn letters(&self, key: &str) -> Result<Vec<char>, String> {
        self.strings(key)?
            .into_iter()
            .map(|w| {
                let mut it = w.chars();
                match (it.next(), it.next()) {
                    (Some(c), None) => Ok(c),
                    _ => Err(format!("label '{key}' takes single characters, got '{w}'")),
                }
            })
            .collect()
    }

    fn letter(&self, key: &str) -> Result<Option<char>, String> {
        Ok(self.letters(key)?.into_iter().next())
    }

    fn flag(&self, key: &str) -> Result<bool, String> {
        match self.field(key)? {
            Json::Bool(b) => Ok(*b),
            _ => Err(format!("label '{key}' must be true or false")),
        }
    }

    fn string(&self, key: &str) -> Result<String, String> {
        match self.field(key)? {
            Json::String(s) if !s.is_empty() => Ok(s.clone()),
            _ => Err(format!("label '{key}' must be a non-empty string")),
        }
    }

    fn string_or_null(&self, key: &str) -> Result<Option<String>, String> {
        match self.field(key)? {
            Json::Null => Ok(None),
            Json::String(s) if !s.is_empty() => Ok(Some(s.clone())),
            _ => Err(format!("label '{key}' must be a non-empty string or null")),
        }
    }

    fn count(&self, key: &str) -> Result<Option<usize>, String> {
        match self.field(key)? {
            Json::Null => Ok(None),
            Json::Number(n) if n.as_u64().is_some() => Ok(n.as_u64().map(|n| n as usize)),
            _ => Err(format!("label '{key}' must be a non-negative integer or null")),
        }
    }

    fn levels(&self, key: &str) -> Result<Vec<Vec<String>>, String> {
        let wrong = || format!("label '{key}' must be a list of lists of strings");
        let Json::Array(tiers) = self.field(key)? else { return Err(wrong()) };
        tiers
            .iter()
            .map(|tier| {
                let Json::Array(items) = tier else { return Err(wrong()) };
                items
                    .iter()
                    .map(|item| match item {
                        Json::String(s) if !s.is_empty() => Ok(s.clone()),
                        _ => Err(wrong()),
                    })
                    .collect()
            })
            .collect()
    }

    fn brackets(&self, open: &str, close: &str, sep: Option<&str>) -> Result<Option<Brackets>, String> {
        let sep = match sep {
            Some(key) => self.head(key)?,
            None => None,
        };
        match (self.head(open)?, self.head(close)?) {
            (Some(open), Some(close)) => Ok(Some(Brackets { open, close, between: sep })),
            (None, None) if sep.is_none() => Ok(None),
            _ => Err(format!("labels '{open}' and '{close}' must be given together")),
        }
    }
}

fn top_object(text: &str) -> Result<serde_json::Map<String, Json>, String> {
    match serde_json::from_str(text).map_err(|e| format!("definition is not valid JSON: {e}"))? {
        Json::Object(map) => Ok(map),
        _ => Err("definition must be a JSON object".to_string()),
    }
}

/// The language name and extensions a definition declares.
pub fn identify(text: &str) -> Result<(String, Vec<String>), String> {
    let map = top_object(text)?;
    let r = Reader(&map);
    Ok((r.string("language")?, r.strings("extensions")?))
}

fn name_like(s: &str, unicode: bool, prefix: Option<char>) -> bool {
    let mut chars = s.chars().peekable();
    if prefix.is_some() && chars.peek().copied() == prefix {
        chars.next();
    }
    let head = |c: char| c == '_' || if unicode { c.is_alphabetic() } else { c.is_ascii_alphabetic() };
    let tail = |c: char| c == '_' || if unicode { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() };
    match chars.next() {
        Some(c) if head(c) => chars.all(tail),
        _ => false,
    }
}

impl Lang {
    pub fn parse(text: &str) -> Result<Lang, String> {
        let map = top_object(text)?;
        let shapes = number_shapes();
        for (_, tag) in &shapes {
            if !map.contains_key(*tag) {
                return Err(format!("missing label '{tag}'"));
            }
        }
        let extensions = shapes_of(EXT_LABELS);
        let mut strange: Vec<&str> = map
            .keys()
            .filter(|k| !k.starts_with('$') && !shapes.iter().chain(&extensions).any(|(_, l)| l == k))
            .map(String::as_str)
            .collect();
        strange.sort();
        if !strange.is_empty() {
            return Err(format!("unknown label(s): {}", strange.join(", ")));
        }
        let r = Reader(&map);
        if r.count("format_version")? != Some(1) {
            return Err("format_version must be 1".to_string());
        }
        for (shape, tag) in &shapes {
            // A label this kernel gives no meaning to is read past, not refused.
            let _ = (shape, tag);
        }

        let name = r.string("language")?;
        let unicode = r.flag("identifier.unicode")?;
        let var_prefix = r.letter("identifier.variable_prefix")?;

        let comment_opens = r.strings("lexical.comment_block.open")?;
        let comment_closes = r.strings("lexical.comment_block.close")?;
        if comment_opens.len() != comment_closes.len() {
            return Err("lexical.comment_block.open and .close must pair up position by position".to_string());
        }
        let quotes = r.letters("lexical.string_quotes")?;
        let raw_quotes = r.letters("lexical.raw_quotes")?;
        if let Some(q) = raw_quotes.iter().find(|q| !quotes.contains(q)) {
            return Err(format!("lexical.raw_quotes lists '{q}', which is not in lexical.string_quotes"));
        }
        let escapes = r.letters("lexical.string_escapes")?;
        if let Some(e) = escapes.iter().find(|e| !matches!(e, 'n' | 't' | 'r' | '0' | '\\') && !quotes.contains(e)) {
            return Err(format!("lexical.string_escapes: unknown escape letter '{e}'"));
        }
        // Each way of writing a number in a base of its own: a digit,
        // one letter, and the base those digits are read in.
        let mut base_prefixes: Vec<(String, u32)> = Vec::new();
        for (tag, base) in [
            ("lexical.number.hex_prefix", 16u32),
            ("ext.lexical.number.binary_prefix", 2),
            ("ext.lexical.number.octal_prefix", 8),
        ] {
            for p in r.strings(tag)? {
                if p.chars().count() != 2 || !p.starts_with(|c: char| c.is_ascii_digit()) {
                    return Err(format!("{tag} must be a digit followed by one letter, got '{p}'"));
                }
                base_prefixes.push((p, base));
            }
        }
        let hex_prefix = r.head("lexical.number.hex_prefix")?;

        let style = match r.string("block.style")?.as_str() {
            "indentation" => Blocks::Indented,
            "braces" => Blocks::Braced,
            "keyword" => Blocks::Worded,
            other => return Err(format!("block.style must be 'indentation', 'braces' or 'keyword', got '{other}'")),
        };
        let postfix = match r.string("syntax.notation")?.as_str() {
            "infix" => false,
            "postfix" => true,
            other => return Err(format!("syntax.notation must be 'infix' or 'postfix', got '{other}'")),
        };
        let openers = r.strings("block.open")?;
        let closers = r.strings("block.close")?;
        let size = r.count("block.indent_size")?;
        let indent = match style {
            Blocks::Indented => {
                if !openers.is_empty() || !closers.is_empty() {
                    return Err("indentation blocks take no block.open or block.close".to_string());
                }
                match size {
                    Some(0) => return Err("block.indent_size must be at least 1".to_string()),
                    Some(n) => n,
                    None => return Err("indentation blocks need block.indent_size".to_string()),
                }
            }
            Blocks::Braced => {
                if openers.is_empty() || openers.len() != closers.len() {
                    return Err("braces need block.open and block.close, paired position by position".to_string());
                }
                size.unwrap_or(4)
            }
            Blocks::Worded => {
                if !openers.is_empty() || closers.is_empty() {
                    return Err("keyword blocks take no block.open and need block.close".to_string());
                }
                size.unwrap_or(4)
            }
        };
        let call = r.brackets("syntax.call.open", "syntax.call.close", Some("syntax.call.separator"))?;
        let call_labels = r.strings("syntax.call.label")?;
        if !call_labels.is_empty() && call.is_none() {
            return Err("syntax.call.label needs syntax.call.open".to_string());
        }
        let index = r.brackets("op.index.open", "op.index.close", None)?;
        let index_text = r.flag("op.index.strings")?;
        if index_text && index.is_none() {
            return Err("op.index.strings needs op.index.open".to_string());
        }

        let tiers = r.levels("op.precedence")?;
        if postfix && !tiers.is_empty() {
            return Err("a postfix language takes no op.precedence".to_string());
        }
        let rights = r.strings("op.right_associative")?;
        let told = r.string_or_null("op.div.result")?;
        let div = match told.as_deref() {
            None | Some("rational") => Action::Div,
            Some("real") | Some("whole_or_real") => Action::DivReal,
            Some(other) => return Err(format!("op.div.result must be 'rational', 'real', 'whole_or_real' or null, got '{other}'")),
        };
        // Dividing one whole number by another gives a whole one where
        // it comes out even, and a real only where it does not.
        let div_stays_whole = told.as_deref() == Some("whole_or_real");
        let tier_of = |lex: &str, from_top: bool| -> Option<u32> {
            if postfix {
                return Some(0);
            }
            let hit = |tier: &Vec<String>| tier.iter().any(|x| x == lex);
            let at = if from_top { tiers.iter().rposition(hit) } else { tiers.iter().position(hit) };
            at.map(|i| i as u32 + 1)
        };
        let mut binary = HashMap::new();
        for (tag, op) in [
            ("op.add", Action::Add), ("op.sub", Action::Sub), ("op.mul", Action::Mul), ("op.div", div),
            ("op.quot", Action::IntDiv), ("op.rem", Action::Mod), ("op.pow", Action::Power), ("op.eq", Action::Eq),
            ("op.ne", Action::Ne), ("op.lt", Action::Lt), ("op.le", Action::Le), ("op.gt", Action::Gt), ("op.ge", Action::Ge),
            ("op.and", Action::And), ("op.or", Action::Or), ("op.concat", Action::Join), ("ext.op.compare", Action::Rank),
            ("ext.op.bit.and", Action::BitBoth), ("ext.op.bit.or", Action::BitEither), ("ext.op.bit.xor", Action::BitOne),
            ("ext.op.bit.left", Action::BitUp), ("ext.op.bit.right", Action::BitDown),
            ("ext.op.in", Action::Contains), ("ext.op.identical", Action::Same), ("ext.op.not_identical", Action::Unsame),
        ] {
            for lex in r.strings(tag)? {
                let tier = tier_of(&lex, false).ok_or_else(|| format!("'{lex}' ({tag}) does not appear in op.precedence"))?;
                let entry = Operator { action: op.clone(), level: tier, right_assoc: rights.contains(&lex) };
                if binary.insert(lex.clone(), entry).is_some() {
                    return Err(format!("'{lex}' is listed under two binary operator labels"));
                }
            }
        }
        let mut unary = HashMap::new();
        for (tag, op) in [("op.not", Action::Not), ("op.negate", Action::Negate), ("ext.op.bit.not", Action::BitTurn)] {
            for lex in r.strings(tag)? {
                let tier = tier_of(&lex, true).ok_or_else(|| format!("'{lex}' ({tag}) does not appear in op.precedence"))?;
                if unary.insert(lex.clone(), Operator { action: op.clone(), level: tier, right_assoc: false }).is_some() {
                    return Err(format!("'{lex}' is listed under two unary operator labels"));
                }
            }
        }
        let ranges = r.strings("op.range")?;
        let pipes = r.strings("op.pipe")?;
        let hushes = r.strings("ext.op.hush")?;
        let mut syntax_tiers = HashMap::new();
        for (tag, list) in [("op.range", &ranges), ("op.pipe", &pipes), ("ext.op.hush", &hushes)] {
            for lex in list {
                let tier = tier_of(lex, false).ok_or_else(|| format!("'{lex}' ({tag}) does not appear in op.precedence"))?;
                syntax_tiers.insert(lex.clone(), tier);
            }
        }
        for lex in tiers.iter().flatten() {
            if !binary.contains_key(lex) && !unary.contains_key(lex) && !syntax_tiers.contains_key(lex) {
                return Err(format!("op.precedence lists '{lex}', which is under no operator tag"));
            }
        }
        if let Some(lex) = rights.iter().find(|lex| !binary.contains_key(*lex)) {
            return Err(format!("op.right_associative lists '{lex}', which is not a binary operator"));
        }

        let lets = r.strings("stmt.let")?;
        let mutables = r.strings("stmt.let.mutable")?;
        let annotation = r.strings("stmt.let.annotation")?;
        let type_first = r.flag("stmt.let.type_first")?;
        if lets.is_empty() && (!mutables.is_empty() || !annotation.is_empty() || type_first) {
            return Err("stmt.let.mutable, stmt.let.annotation and stmt.let.type_first need stmt.let".to_string());
        }
        let ifs = r.strings("stmt.if")?;
        let elifs = r.strings("stmt.elif")?;
        let elses = r.strings("stmt.else")?;
        if ifs.is_empty() && (!elifs.is_empty() || !elses.is_empty()) {
            return Err("stmt.elif and stmt.else need stmt.if".to_string());
        }
        let fors = r.strings("stmt.for")?;
        let ins = r.strings("stmt.for.in")?;
        if fors.is_empty() != ins.is_empty() && !(postfix && ins.is_empty()) {
            return Err("stmt.for and stmt.for.in must be given together".to_string());
        }
        let functions = r.strings("stmt.function")?;
        // The extension mark stands beside the core one: a language whose
        // return types the porter cannot spell says it here instead.
        let mut returns_marks = r.strings("stmt.function.returns")?;
        returns_marks.extend(r.strings("ext.stmt.function.returns")?);
        let result_by_name = r.flag("stmt.function.result_by_name")?;
        if result_by_name && functions.is_empty() {
            return Err("stmt.function.result_by_name needs stmt.function".to_string());
        }
        if type_first {
            for (tag, list) in [
                ("stmt.let.mutable", &mutables), ("stmt.let.annotation", &annotation),
                ("stmt.function", &functions), ("stmt.function.returns", &returns_marks),
            ] {
                if !list.is_empty() {
                    return Err(format!("stmt.let.type_first leaves no place for {tag}; leave it empty"));
                }
            }
        }

        let stack_lists = [
            "stack.dup", "stack.drop", "stack.swap", "stack.over", "stack.rot", "stack.eval",
            "stack.program.open", "stack.program.close",
        ]
        .iter()
        .map(|tag| r.strings(tag))
        .collect::<Result<Vec<_>, _>>()?;
        if !postfix && stack_lists.iter().any(|l| !l.is_empty()) {
            return Err("the stack.* labels need syntax.notation 'postfix'".to_string());
        }
        if stack_lists[6].len() != stack_lists[7].len() {
            return Err("stack.program.open and .close must pair up position by position".to_string());
        }
        let mut stack_lists = stack_lists.into_iter();
        let mut next_list = || stack_lists.next().expect("eight stack lists");

        let mut natives = HashMap::new();
        for (tag, native) in [
            ("ext.builtin.sum", Builtin::Sum), ("ext.builtin.list", Builtin::List), ("ext.builtin.any", Builtin::Any),
            ("builtin.emit", Builtin::Echo), ("builtin.print", Builtin::Say), ("builtin.write", Builtin::Out),
            ("builtin.len", Builtin::Length), ("builtin.char_at", Builtin::CharAtIndex), ("builtin.ord", Builtin::CodeOf),
            ("builtin.chr", Builtin::CharOf), ("builtin.typeof", Builtin::SortOf), ("builtin.error", Builtin::Raise),
            ("builtin.extern", Builtin::External), ("builtin.range", Builtin::Span), ("builtin.real", Builtin::MakeReal),
            ("builtin.precision", Builtin::Places), ("builtin.to_string", Builtin::ToText),
            ("builtin.to_int", Builtin::ToInt), ("builtin.to_real", Builtin::AsReal), ("builtin.num", Builtin::Numer),
            ("builtin.den", Builtin::Denom), ("builtin.push", Builtin::Append), ("builtin.get", Builtin::Fetch),
            ("builtin.put", Builtin::Replace), ("ext.builtin.echo", Builtin::Tell), ("ext.builtin.define", Builtin::Define),
            ("ext.builtin.var_dump", Builtin::Dump), ("ext.builtin.array", Builtin::Pack),
            ("ext.builtin.print_r", Builtin::Layout), ("ext.builtin.unset", Builtin::Erase), ("ext.builtin.array.front", Builtin::Lead),
            ("ext.builtin.isset", Builtin::Held), ("ext.builtin.empty", Builtin::Hollow),
            ("ext.builtin.exit", Builtin::Leave),
            ("ext.builtin.args.all", Builtin::Given), ("ext.builtin.args.count", Builtin::GivenCount),
            ("ext.builtin.args.at", Builtin::GivenAt), ("ext.builtin.time_limit", Builtin::TimeLimit),
            ("ext.builtin.eval", Builtin::Eval), ("ext.builtin.include", Builtin::Include), ("ext.builtin.include.once", Builtin::IncludeOnce),
            ("ext.builtin.output.hold", Builtin::HoldOut), ("ext.builtin.output.held", Builtin::HeldOut),
            ("ext.builtin.output.drop", Builtin::DropOut), ("ext.builtin.output.depth", Builtin::DeepOut),
            ("ext.builtin.output.begun", Builtin::OutBegun),
            ("ext.builtin.at_end", Builtin::WhenDone), ("ext.builtin.complaint.handler", Builtin::Complainer), ("ext.builtin.complaint.say", Builtin::Complain),
            ("ext.builtin.calls", Builtin::Calls),
            ("ext.builtin.uncaught", Builtin::Untaken),
            ("ext.builtin.classes", Builtin::ClassesBound), ("ext.builtin.routines", Builtin::RoutinesBound), ("ext.builtin.spelled", Builtin::Spelled), ("ext.builtin.class.methods", Builtin::ClassMethods), ("ext.builtin.class.properties", Builtin::ClassProperties),
            ("ext.builtin.class.beneath", Builtin::ClassBeneath), ("ext.builtin.math", Builtin::Math),
            ("ext.builtin.program.namespace", Builtin::ProgramNamespace),
            ("ext.builtin.member.get", Builtin::MemberGet),
            ("ext.builtin.member.set", Builtin::MemberSet),
            ("ext.builtin.instance", Builtin::InstanceOf),
            ("ext.builtin.module.load", Builtin::ModuleLoad),
            ("ext.builtin.copy", Builtin::CopyValue),
            ("ext.builtin.call.outcome", Builtin::CallOutcome),
            ("ext.builtin.clock", Builtin::Clock),
            ("ext.builtin.room.used", Builtin::RoomUsed), ("ext.builtin.room.most", Builtin::RoomMost),
            ("ext.builtin.room.most.forget", Builtin::RoomForget), ("ext.builtin.room.limit", Builtin::RoomLimit),
            ("ext.builtin.file.read", Builtin::FileRead), ("ext.builtin.file.write", Builtin::FileWrite),
            ("ext.builtin.file.exists", Builtin::FileThere), ("ext.builtin.file.remove", Builtin::FileGone),
            ("ext.builtin.shell", Builtin::ShellSaid),
            ("ext.builtin.net.ask", Builtin::NetAsk), ("ext.builtin.wait", Builtin::Waited),
            ("ext.builtin.run.begin", Builtin::RunBegin), ("ext.builtin.run.end", Builtin::RunEnd),
        ] {
            for lex in r.strings(tag)? {
                let begins = lex.chars().next().map_or(false, |c| c == '_' || c.is_alphabetic());
                if !begins || lex.chars().any(|c| c.is_whitespace() || quotes.contains(&c)) {
                    return Err(format!("builtin name '{lex}' must begin like an identifier and hold no spaces or quotes"));
                }
                if natives.insert(lex.clone(), native).is_some() {
                    return Err(format!("'{lex}' is listed under two builtin labels"));
                }
            }
        }

        let mut kind_names = Vec::new();
        for (tag, kind) in [
            ("system.kind.integer", Sort::Integer), ("system.kind.rational", Sort::Rational),
            ("system.kind.real", Sort::Real), ("system.kind.string", Sort::Text),
            ("system.kind.boolean", Sort::Boolean), ("system.kind.array", Sort::Array),
            ("system.kind.null", Sort::Null),
        ] {
            if let Some(n) = r.head(tag)? {
                kind_names.push((n, kind));
            }
        }
        let args_name = r.head("system.args")?;
        let memo_name = r.head("system.memoization")?;
        let precision_name = r.head("system.real_default_precision")?;
        let entry_name = r.head("system.entry")?;
        let system_names = [&args_name, &memo_name, &precision_name, &entry_name].into_iter().flatten();
        for n in system_names.chain(kind_names.iter().map(|(n, _)| n)) {
            if !name_like(n, unicode, var_prefix) {
                return Err(format!("system name '{n}' must be shaped like an identifier"));
            }
        }

        // A language with a word for any kind of complaint says where
        // the complaint happened, so the lines are worth marking.
        let tells_complaints = ["ext.system.complaint.warning", "ext.system.complaint.notice", "ext.system.complaint.deprecated", "ext.system.complaint.fatal"]
            .into_iter()
            .try_fold(false, |found, tag| Ok::<bool, String>(found || r.head(tag)?.is_some()))?;

        // A language with an operator for being the very same means
        // something looser by being equal.
        let tells_same = binary.values().any(|op| matches!(op.action, Action::Same));

        // A language that can read what a call was given is one whose
        // calls may give more than a routine names.
        let reads_arguments = natives.values().any(|b| matches!(b, Builtin::Given | Builtin::GivenCount | Builtin::GivenAt));

        let mut prefix: String = name.chars().take(1).flat_map(char::to_uppercase).collect();
        prefix.push_str(name.get(1..).unwrap_or(""));
        prefix.push_str("Error");

        let mut lang = Lang {
            ident: name,
            extensions: r.strings("extensions")?,
            banner: prefix,
            with_as: r.strings("ext.stmt.with.as")?,
            with_unready: r.strings("ext.stmt.with.unready")?,
            yield_from: r.strings("ext.stmt.yield.from")?,
            member_pipes: r.flag("ext.op.member.pipes")?,
            tuple_unready: r.strings("ext.op.tuple.unready")?,
            bytes_unready: r.strings("ext.lexical.string.prefix.bytes.unready")?,
            format_unready: r.strings("ext.lexical.string.prefix.format.unready")?,
            identity_unready: r.strings("ext.op.identical.unsupported")?,
            in_values: r.strings("ext.op.in")?,
            in_not: r.strings("ext.op.in.negated")?,
            in_unready: r.strings("ext.op.in.unsupported")?,
            if_else: r.strings("ext.op.if_else")?,
            assign_chain: r.flag("ext.stmt.assign.chain")?,
            deferred_escapes: r.letters("ext.lexical.escape.deferred")?,
            line_comments: r.strings("lexical.comment_line")?,
            block_comments: comment_opens.into_iter().zip(comment_closes).collect(),
            quotes,
            raw_quotes,
            long_quotes: r.strings("ext.lexical.string.long")?,
            raw_prefixes: r.letters("ext.lexical.string.prefix.raw")?,
            byte_prefixes: r.letters("ext.lexical.string.prefix.bytes")?,
            plain_prefixes: r.letters("ext.lexical.string.prefix.plain")?,
            format_prefixes: r.letters("ext.lexical.string.prefix.format")?,
            adjacent_strings: r.flag("ext.lexical.string.adjacent")?,
            string_amiss: r.head("ext.lexical.string.amiss")?,
            byte_digits: r.count("ext.lexical.escape.byte.digits")?,
            codepoint_digits: r.count("ext.lexical.escape.codepoint.digits")?,
            wide_letter: r.letter("ext.lexical.escape.codepoint.wide")?,
            wide_digits: r.count("ext.lexical.escape.codepoint.wide.digits")?,
            named_letter: r.letter("ext.lexical.escape.named")?,
            escape_unavailable: r.head("ext.lexical.escape.unavailable")?,
            escape_letters: escapes,
            control_escapes: r.letters("ext.lexical.escape.controls")?,
            continued_strings: r.flag("ext.lexical.escape.continued")?,
            codepoint_letter: r.letter("ext.lexical.escape.codepoint")?,
            codepoint_open: r.letter("ext.lexical.escape.codepoint.open")?,
            codepoint_close: r.letter("ext.lexical.escape.codepoint.close")?,
            codepoint_amiss: r.head("ext.lexical.escape.codepoint.amiss")?,
            codepoint_beyond: r.head("ext.lexical.escape.codepoint.beyond")?,
            byte_letter: r.letter("ext.lexical.escape.byte")?,
            number_amiss: r.head("ext.lexical.number.amiss")?,
            prologue: r.head("lexical.prologue")?,
            point: r.letter("lexical.number.decimal_point")?,
            base_mark: r.letter("lexical.number.base_marker")?,
            exponent_mark: r.letter("lexical.number.exponent_marker")?,
            hex_prefix,
            base_prefixes,
            octal_lead: r.flag("ext.lexical.number.octal_lead")?,
            digit_separators: r.letters("ext.lexical.number.separator")?,
            integer_bits: r.count("ext.system.integer.bits")?,
            real_bits: r.count("ext.system.real.bits")?,
            real_digits: r.count("ext.system.real.digits")?,
            figures_binding: r.head("ext.system.real.figures")?,
            figures_shown_binding: r.head("ext.system.real.figures.shown")?,
            unicode_names: unicode,
            sigil: var_prefix,
            keywords_folded: r.flag("lexical.keywords_case_insensitive")?,
            names_folded: r.flag("identifier.case_insensitive")?,
            quote_for_names: r.letter("lexical.name_quote")?,
            symbols: Vec::new(),
            keywords: HashSet::new(),
            blocks: style,
            rpn: postfix,
            indent_width: indent,
            block_opens: openers,
            block_closes: closers,
            block_intros: r.strings("block.intro")?,
            stmt_ends: r.strings("stmt.terminator")?.into_iter().chain(r.strings("ext.stmt.terminator")?).collect(),
            stmt_separators: r.strings("ext.stmt.separator")?,
            grouping: r.brackets("syntax.group.open", "syntax.group.close", None)?,
            calling: call,
            argument_labels: call_labels,
            array_brackets: r.brackets("syntax.array.open", "syntax.array.close", Some("syntax.array.separator"))?,
            map_brackets: r.brackets("syntax.map.open", "syntax.map.close", Some("syntax.map.separator"))?,
            pair_mark: r.head("syntax.map.pair")?,
            index_brackets: index,
            slice_ellipsis: r.strings("ext.op.index.slice.ellipsis")?,
            slice_marks: r.strings("ext.op.index.slice")?,
            slice_zero: r.head("ext.op.index.slice.zero")?,
            slice_bounds: r.head("ext.op.index.slice.bounds")?,
            slice_unsupported: r.head("ext.op.index.slice.unsupported")?,
            slice_assign: r.head("ext.op.index.slice.assign")?,
            slice_length: r.strings("ext.op.index.slice.length")?,
            slice_detached: r.head("ext.op.index.slice.detached")?,
            text_indexable: index_text,
            true_words: r.strings("literal.true")?,
            false_words: r.strings("literal.false")?,
            null_words: r.strings("literal.null")?,
            null_silent: r.flag("literal.null.silent")?,
            dyadic: binary,
            monadic: unary,
            pipe_words: pipes,
            range_marks: ranges,
            precedence: syntax_tiers,
            assign_words: r.strings("stmt.assign")?,
            let_words: lets,
            mutable_words: mutables,
            type_marks: annotation,
            annotation_marks: r.strings("ext.stmt.annotation")?,
            annotation_amiss: r.head("ext.stmt.annotation.amiss")?,
            annotation_target_unready: r.head("ext.stmt.annotation.target.unready")?,
            types_first: type_first,
            if_words: ifs,
            elif_words: elifs,
            else_words: elses,
            while_words: r.strings("stmt.while")?,
            until_words: r.strings("stmt.until")?,
            for_words: fors,
            in_words: ins,
            return_words: r.strings("stmt.return")?,
            break_words: r.strings("stmt.break")?,
            continue_words: r.strings("stmt.continue")?,
            function_words: functions,
            return_marks: returns_marks,
            named_result: result_by_name,
            pass_words: r.strings("stmt.pass")?,
            dup_words: next_list(),
            drop_words: next_list(),
            swap_words: next_list(),
            over_words: next_list(),
            rot_words: next_list(),
            eval_words: next_list(),
            quote_open: next_list(),
            quote_close: next_list(),
            builtins: natives,
            holes: r.strings("builtin.print.placeholder")?,
            args_binding: args_name,
            args_list: r.head("ext.system.args.list")?,
            args_count: r.head("ext.system.args.count")?,
            memo_binding: memo_name,
            precision_binding: precision_name,
            entry_binding: entry_name,
            sort_bindings: kind_names,
            brief_kinds: r.strings("ext.system.kind.brief")?,
            object_kind: r.head("ext.system.kind.object")?,
            loose_kinds: r.strings("ext.system.kind.loose")?,
            kind_spelled: r.flag("ext.system.kind.spelled")?,
            spare_args: reads_arguments,
            assign_gives_value: r.flag("ext.op.assign.value")?,
            plain_keys: r.flag("ext.op.index.plain_keys")?,
            loose_equality: tells_same && r.strings("ext.op.identical.negated")?.is_empty(),
            complaint_words: {
                let named = [
                    (Complaint::Warning, "ext.system.complaint.warning"),
                    (Complaint::Notice, "ext.system.complaint.notice"),
                    (Complaint::Deprecated, "ext.system.complaint.deprecated"),
                    (Complaint::Fatal, "ext.system.complaint.fatal"),
                ];
                let mut found = Vec::new();
                for (kind, tag) in named {
                    if let Some(word) = r.head(tag)? {
                        found.push((kind, word));
                    }
                }
                found
            },
            markup_setting: r.head("ext.system.complaint.markup.setting")?,
            markup_kind: r.about_two("ext.system.complaint.markup.kind")?,
            markup_place: r.around("ext.system.complaint.markup.place")?,
            markup_line: r.around("ext.system.complaint.markup.line")?,
            markup_page: r.about_two("ext.system.complaint.markup.reference")?,
            pages_setting: r.head("ext.system.complaint.reference.setting")?,
            page_named: r.around("ext.system.complaint.reference.page")?,
            page_mark: r.around("ext.system.complaint.reference.mark")?,
            line_binding: r.head("ext.system.source.line")?,
            routine_binding: r.head("ext.system.source.routine")?,
            class_binding: r.head("ext.system.source.class")?,
            method_binding: r.head("ext.system.source.method")?,
            fault_class: r.head("ext.system.fault.class")?,
            fault_arithmetic: r.head("ext.system.fault.class.arithmetic")?,
            fault_division: r.head("ext.system.fault.class.division")?,
            fault_kind: r.head("ext.system.fault.class.kind")?,
            fault_value: r.head("ext.system.fault.class.value")?,
            fault_walk: r.head("ext.system.fault.class.walk")?,
            operand_fault: r.head("ext.system.fault.operands")?,
            fault_modulo: r.head("ext.system.fault.modulo")?,
            fault_shift: r.head("ext.system.fault.shift")?,
            shift_by_number: r.flag("ext.op.bit.shift.numbers")?,
            div_stays_whole,
            mod_whole: r.flag("op.mod.whole")?,
            step_up_text: r.head("ext.op.increment.text")?,
            step_down_text: r.head("ext.op.decrement.text")?,
            warns_of_unwritten: r.head("ext.system.complaint.warning")?.is_some(),
            tells_place: tells_complaints,
            source_bindings: {
                let named = [("file", "ext.system.source.file"), ("directory", "ext.system.source.directory"), ("runner", "ext.system.runner")];
                let mut found = Vec::new();
                for (part, tag) in named {
                    if let Some(word) = r.head(tag)? {
                        found.push((part.to_string(), word));
                    }
                }
                found
            },
            epilogue: r.strings("ext.lexical.epilogue")?,
            prologue_echo: r.head("ext.lexical.prologue.echo")?,
            prologue_folded: r.flag("ext.lexical.prologue.folded")?,
            bare_calls: r.flag("ext.syntax.call.bare")?,
            writes_as_operator: r.flag("ext.builtin.write.operator")?,
            increments: r.strings("ext.op.increment")?,
            decrements: r.strings("ext.op.decrement")?,
            interpolating: r.letters("ext.lexical.interpolating_quotes")?,
            heredoc: r.head("ext.lexical.heredoc")?,
            octal_escapes: r.flag("ext.lexical.escape.octal")?,
            prologue_brief: r.head("ext.lexical.prologue.brief")?,
            prologue_brief_setting: r.head("ext.lexical.prologue.brief.setting")?,
            text_is_bytes: r.flag("ext.system.text.bytes")?,
            concat: r.head("op.concat")?,
            foreach_words: r.strings("stmt.foreach")?,
            foreach_as_words: r.strings("stmt.foreach.as")?,
            c_for_words: r.strings("ext.stmt.for.c")?,
            compound: HashMap::new(),
            static_words: r.strings("ext.stmt.static")?,
            tuple_marks: r.strings("ext.op.tuple")?,
            class_bases_open: r.strings("ext.stmt.class.bases.open")?,
            class_bases_close: r.strings("ext.stmt.class.bases.close")?,
            class_unready: r.strings("ext.stmt.class.unready")?,
            del_words: r.strings("ext.stmt.del")?,
            nonlocal_words: r.strings("ext.stmt.nonlocal")?,
            nonlocal_unrun: r.strings("ext.stmt.nonlocal.unrun")?,
            with_enter: r.head("ext.stmt.with.enter")?,
            with_leave: r.head("ext.stmt.with.leave")?,
            with_words: r.strings("ext.stmt.with")?,
            with_as_words: r.strings("ext.stmt.with.as")?,
            yield_words: r.strings("ext.stmt.yield")?,
            yield_from_words: r.strings("ext.stmt.yield.from")?,
            yield_unrun: r.strings("ext.stmt.yield.unrun")?,
            scope_unready: r.strings("ext.system.scope.unready")?,
            global_words: r.strings("ext.stmt.global")?,
            await_words: r.strings("ext.op.await")?,
            async_words: r.strings("ext.stmt.async")?,
            loop_else: r.flag("ext.stmt.loop.else")?,
            binding_unrun: r.head("ext.stmt.binding.unrun")?.unwrap_or_default(),
            del_unrun: r.head("ext.stmt.del.unrun")?.unwrap_or_default(),
            import_values: r.flag("ext.stmt.import.value")?,
            import_missing: r.strings("ext.stmt.import.missing")?,
            import_member_missing: r.strings("ext.stmt.import.member.missing")?,
            import_relative_unready: r.head("ext.stmt.import.relative.unready")?.unwrap_or_default(),
            import_words: r.strings("ext.stmt.import")?,
            import_from_words: r.strings("ext.stmt.import.from")?,
            import_as_words: r.strings("ext.stmt.import.as")?,
            math_floating: r.flag("ext.builtin.math.floating")?,
            module_cache: r.strings("ext.system.module.cache")?,
            module_names: r.strings("ext.system.module.name")?,
            decorator_words: r.strings("ext.stmt.decorator")?,
            decorator_amiss: r.head("ext.stmt.decorator.amiss")?,
            const_words: r.strings("ext.stmt.const")?,
            switch_words: r.strings("ext.stmt.switch")?,
            case_words: r.strings("ext.stmt.case")?,
            default_words: r.strings("ext.stmt.default")?,
            case_marks: r.strings("ext.stmt.case.mark")?,
            case_mark_instead: r.head("ext.stmt.case.mark.instead")?,
            ternary: match r.strings("ext.op.ternary")?.as_slice() {
                [] => None,
                [q, m] => Some((q.clone(), m.clone())),
                _ => return Err("ext.op.ternary takes exactly two signs, the question and the mark".to_string()),
            },
            lone_stmt: r.flag("ext.block.lone_statement")?,
            hoisted: r.flag("ext.stmt.function.hoisted")?,
            routines_outermost: r.flag("ext.stmt.function.outermost")?,
            own_names: r.flag("ext.stmt.function.own_names")?,
            static_read_in: r.flag("ext.stmt.static.read_in")?,
            body_binding: r.head("ext.system.request.body")?,
            amiss_binding: r.head("ext.system.request.amiss")?,
            amiss_words: {
                let kinds = [
                    ("boundary", "ext.system.request.amiss.boundary"),
                    ("boundary.wrong", "ext.system.request.amiss.boundary.wrong"),
                    ("part", "ext.system.request.amiss.part"),
                    ("body.large", "ext.system.request.amiss.body.large"),
                ];
                let mut said = Vec::new();
                for (kind, tag) in kinds {
                    if let Some(words) = r.head(tag)? {
                        said.push((kind, words));
                    }
                }
                said
            },
            exponent_letters: r.letters("ext.lexical.number.exponent")?,
            plus_words: r.strings("ext.op.plus")?,
            if_else_words: r.strings("ext.op.if_else")?,
            lambda_words: r.strings("ext.op.lambda")?,
            lambda_unsupported: r.head("ext.op.lambda.unsupported")?,
            lambda_enclosing: r.head("ext.op.lambda.enclosing")?,
            identity_not: r.strings("ext.op.identical.negated")?,
            identity_unsupported: r.head("ext.op.identical.unsupported")?,
            membership_words: r.strings("ext.op.in")?,
            membership_not: r.strings("ext.op.in.negated")?,
            membership_unsupported: r.head("ext.op.in.unsupported")?,
            chained_comparisons: r.flag("ext.op.compare.chained")?,
            expression_assign: r.strings("ext.op.assign.expression")?,
            ellipsis_words: r.strings("ext.literal.ellipsis")?,
            rem_formats_text: r.flag("ext.op.rem.formats_text")?,
            format_unsupported: r.head("ext.op.rem.format.unsupported")?,
            format_arguments: r.head("ext.op.rem.format.arguments")?,

            hush_words: hushes,
            naming_words: r.strings("ext.op.name_by_value")?,
            casts_kinds: r.flag("ext.op.cast")?,
            members_by_value: r.flag("ext.op.member.by_value")?,
            text_places: r.flag("ext.op.index.text")?,
            text_place_first: r.head("ext.op.index.text.first")?,
            define_scoped: r.head("ext.builtin.define.class_constant")?,
            globals_words: r.strings("ext.system.globals")?,
            unshared_written: r.strings("ext.op.reference.unshared.written")?,
            unshared_given: r.strings("ext.op.reference.unshared.given")?,
            unshared_handed: r.strings("ext.op.reference.unshared.handed")?,
            instead_mark: r.head("ext.stmt.block.instead")?,
            instead_closes: r.strings("ext.stmt.block.instead.close")?,
            do_words: r.strings("ext.stmt.do")?,
            spelled_stands: r.flag("ext.op.spelled")?,
            classes_folded: r.flag("ext.system.class.folded")?,
            terminator_only: r.flag("ext.stmt.terminator.only")?,
            reading_word: r.head("ext.system.complaint.reading")?,
            reading_unexpected: r.head("ext.system.reading.unexpected")?,
            reading_character: r.head("ext.system.reading.unexpected.character")?,
            unclosed_words: r.around("ext.system.reading.unclosed")?,
            unclosed_line: r.head("ext.system.reading.unclosed.line")?,
            mismatch_words: r.around("ext.system.reading.unclosed.mismatch")?,
            unmatched_words: r.around("ext.system.reading.unmatched")?,
            fault_reading: r.head("ext.system.fault.class.reading")?,
            eval_place: r.around("ext.builtin.eval.place")?,
            include_demanded: r.strings("ext.builtin.include.demanded")?,
            include_demanded_missing: r.around("ext.builtin.include.demanded.missing")?,
            woven_index_words: r.head("ext.lexical.interpolating.index.amiss")?,
            flags_count: r.flag("system.flag.counts")?,
            args_outside_all: r.head("ext.builtin.args.all.outside")?,
            args_outside_count: r.head("ext.builtin.args.count.outside")?,
            args_outside_at: r.head("ext.builtin.args.at.outside")?,
            args_below: r.head("ext.builtin.args.at.below")?,
            args_beyond: r.head("ext.builtin.args.at.beyond")?,
            unpack_words: r.strings("ext.stmt.unpack")?,
            unpack_rest: r.strings("ext.stmt.unpack.rest")?,
            unpack_short: r.head("ext.stmt.unpack.short")?,
            unpack_long: r.head("ext.stmt.unpack.long")?,
            unpack_unwalkable: r.head("ext.stmt.unpack.unwalkable")?,
            unpack_amiss: r.head("ext.stmt.unpack.amiss")?,
            makes_places: r.flag("ext.op.index.makes")?,
            untrue_text: r.strings("ext.system.untrue.text")?,
            untrue_empty: r.flag("ext.system.untrue.empty_array")?,
            break_levels: r.flag("ext.stmt.break.levels")?,
            template: r.flag("ext.lexical.template")?,
            append_index: r.flag("ext.op.index.append")?,
            for_collections: r.flag("ext.stmt.for.collection")?,
            comprehension_async: r.strings("ext.op.comprehension.async")?,
            comprehension_async_unavailable: r.strings("ext.op.comprehension.async.unavailable")?,
            comprehension_target_unavailable: r.strings("ext.op.comprehension.target.unavailable")?,
            sum_non_number: r.strings("ext.builtin.sum.non_number")?,
            range_non_integer: r.strings("ext.builtin.range.non_integer")?,
            range_zero_step: r.strings("ext.builtin.range.zero_step")?,

            comprehension_for: r.strings("ext.op.comprehension.for")?,
            comprehension_in: r.strings("ext.op.comprehension.in")?,
            comprehension_if: r.strings("ext.op.comprehension.if")?,
            set_literals: r.flag("ext.syntax.set")?,
            array_spread: r.strings("ext.syntax.array.spread")?,
            map_spread: r.strings("ext.syntax.map.spread")?,
            collection_unwalkable: r.strings("ext.syntax.collection.unwalkable")?,
            spread_unmapped: r.strings("ext.syntax.map.spread.unmapped")?,
            comprehension_unpack_amiss: r.strings("ext.op.comprehension.unpack.amiss")?,

            class_words: r.strings("ext.stmt.class")?,
            bases_open: r.head("ext.stmt.class.bases.open")?,
            bases_close: r.head("ext.stmt.class.bases.close")?,
            explicit_this: r.flag("ext.stmt.class.this.explicit")?,
            extends_words: r.strings("ext.stmt.class.extends")?,
            new_words: r.strings("ext.stmt.class.new")?,
            this_word: r.head("ext.stmt.class.this")?,
            constructor: r.head("ext.stmt.class.constructor")?,
            destructor: r.head("ext.stmt.class.destructor")?,
            reader: r.head("ext.stmt.class.reader")?,
            writer: r.head("ext.stmt.class.writer")?,
            caller: r.head("ext.stmt.class.caller")?,
            walker_class: r.head("ext.op.walk.class")?,
            walk_rewind: r.head("ext.op.walk.rewind")?,
            walk_more: r.head("ext.op.walk.more")?,
            walk_this: r.head("ext.op.walk.this")?,
            walk_key: r.head("ext.op.walk.key")?,
            walk_onward: r.head("ext.op.walk.onward")?,
            giver_class: r.head("ext.op.walk.giver.class")?,
            walk_giver: r.head("ext.op.walk.giver")?,
            giver_unwalkable: match r.strings("ext.op.walk.giver.unwalkable")?.as_slice() {
                [] => None,
                [before, after] => Some((before.clone(), after.clone())),
                _ => return Err("ext.op.walk.giver.unwalkable takes exactly two pieces, what is said before the name and what is said after".to_string()),
            },
            walk_no_cell: r.head("ext.op.walk.no_cell")?,
            walk_key_no_cell: r.head("ext.op.walk.key.no_cell")?,
            walk_alive: r.flag("ext.op.walk.live")?,
            modifier_words: r.strings("ext.stmt.class.modifier")?,
            hidden_words: r.strings("ext.stmt.class.hidden")?,
            guarded_words: r.strings("ext.stmt.class.guarded")?,
            shared_words: r.strings("ext.stmt.class.shared")?,
            member_mark: r.head("ext.op.member")?,
            scope_mark: r.head("ext.op.scope")?,
            instanceof_words: r.strings("ext.op.instanceof")?,
            otherwise_mark: r.head("ext.op.otherwise")?,
            trait_words: r.strings("ext.stmt.class.trait")?,
            uses_words: r.strings("ext.stmt.class.uses")?,
            carries_words: r.strings("ext.stmt.function.carries")?,
            bind_names: r.flag("ext.syntax.call.bind_names")?,
            carries_pairs: r.strings("ext.stmt.function.carries.pairs")?,
            keyword_only: r.strings("ext.stmt.function.keyword_only")?,
            positional_only: r.strings("ext.stmt.function.positional_only")?,
            call_spread: r.strings("ext.syntax.call.spread")?,
            call_spread_pairs: r.strings("ext.syntax.call.spread.pairs")?,
            call_amiss: r.strings("ext.syntax.call.amiss")?,
            call_missing: r.strings("ext.syntax.call.amiss.missing")?,
            call_unknown: r.strings("ext.syntax.call.amiss.unknown")?,
            call_duplicate: r.strings("ext.syntax.call.amiss.duplicate")?,
            call_builtin_amiss: r.strings("ext.syntax.call.amiss.builtin")?,
            print_sep: r.strings("ext.builtin.print.sep")?,
            print_end: r.strings("ext.builtin.print.end")?,
            print_file: r.strings("ext.builtin.print.file")?,
            print_flush: r.strings("ext.builtin.print.flush")?,
            print_file_error: r.strings("ext.builtin.print.file.error")?,
            print_file_output: r.strings("ext.builtin.print.file.output")?,
            print_file_unready: r.strings("ext.builtin.print.file.unready")?,
            print_sep_amiss: r.strings("ext.builtin.print.sep.amiss")?,
            print_end_amiss: r.strings("ext.builtin.print.end.amiss")?,
            to_int_base: r.strings("ext.builtin.to_int.base")?,
            to_int_base_amiss: r.strings("ext.builtin.to_int.base.amiss")?,
            to_int_text_amiss: r.strings("ext.builtin.to_int.text.amiss")?,
            to_int_text_required: r.strings("ext.builtin.to_int.text.required")?,
            to_real_text: r.flag("ext.builtin.to_real.text")?,
            to_real_text_amiss: r.strings("ext.builtin.to_real.text.amiss")?,
            to_string_object: r.strings("ext.builtin.to_string.object")?,
            to_string_encoding: r.strings("ext.builtin.to_string.encoding")?,
            to_string_errors: r.strings("ext.builtin.to_string.errors")?,
            to_string_unready: r.strings("ext.builtin.to_string.unready")?,
            range_value: r.flag("ext.builtin.range.value")?,
            range_zero: r.strings("ext.builtin.range.zero")?,
            range_integer: r.strings("ext.builtin.range.integer")?,
            range_index: r.strings("ext.builtin.range.index")?,
            spread_amiss: r.strings("ext.syntax.call.spread.amiss")?,
            spread_pairs_amiss: r.strings("ext.syntax.call.spread.pairs.amiss")?,
            defaults_amiss: r.strings("ext.stmt.function.defaults.amiss")?,
            parameters_amiss: r.strings("ext.stmt.function.parameters.amiss")?,

            short_function: match r.strings("ext.stmt.function.short")?.as_slice() {
                [] => None,
                [word, mark] => Some((word.clone(), mark.clone())),
                _ => return Err("ext.stmt.function.short takes exactly two words, the one it opens with and the mark before its body".to_string()),
            },
            uses_alias: r.strings("ext.stmt.class.uses.alias")?,
            interface_words: r.strings("ext.stmt.class.interface")?,
            implements_words: r.strings("ext.stmt.class.implements")?,
            parent_words: r.strings("ext.stmt.class.parent")?,
            self_words: r.strings("ext.stmt.class.self")?,
            name_leads: r.letters("ext.lexical.name_lead")?,
            assert_words: r.strings("ext.stmt.assert")?,
            assert_kind: r.head("ext.stmt.assert.kind")?,
            catch_as: r.strings("ext.stmt.catch.as")?,
            catch_invalid: r.head("ext.stmt.catch.invalid")?,
            catch_tuple_open: r.head("ext.stmt.catch.tuple.open")?,
            catch_tuple_close: r.head("ext.stmt.catch.tuple.close")?,
            catch_group: r.head("ext.stmt.catch.group")?,
            catch_group_unsupported: r.head("ext.stmt.catch.group.unsupported")?,
            try_else: r.flag("ext.stmt.try.else")?,
            throw_from: r.strings("ext.stmt.throw.from")?,
            throw_empty: r.head("ext.stmt.throw.empty")?,
            try_words: r.strings("ext.stmt.try")?,
            catch_words: r.strings("ext.stmt.catch")?,
            finally_words: r.strings("ext.stmt.finally")?,
            throw_words: r.strings("ext.stmt.throw")?,
            catch_between: r.head("ext.stmt.catch.separator")?,
            reference_mark: r.head("ext.op.reference")?,
            absent_index: r.flag("ext.op.index.absent")?,
            scalar_index: r.head("ext.op.index.scalar")?,
            nothing_index: r.head("ext.op.index.nothing")?,
            request_bindings: {
                let groups = [
                    ("GET", "ext.system.request.query"), ("POST", "ext.system.request.form"),
                    ("COOKIE", "ext.system.request.cookies"), ("SERVER", "ext.system.request.server"),
                    ("ENV", "ext.system.request.env"), ("FILES", "ext.system.request.files"),
                    ("ALL", "ext.system.request.all"), ("SETTINGS", "ext.system.request.settings"),
                ];
                let mut named = Vec::new();
                for (group, tag) in groups {
                    if let Some(name) = r.head(tag)? {
                        named.push((group.to_string(), name));
                    }
                }
                named
            },
        };
        if !lang.try_words.is_empty() && lang.catch_words.is_empty() {
            return Err("ext.stmt.try needs ext.stmt.catch".to_string());
        }
        if !lang.class_words.is_empty() && (lang.member_mark.is_none() || (lang.new_words.is_empty() && !lang.explicit_this)) {
            return Err("ext.stmt.class needs ext.op.member and ext.stmt.class.new".to_string());
        }
        if !lang.foreach_words.is_empty() && lang.foreach_as_words.is_empty() {
            return Err("stmt.foreach needs stmt.foreach.as".to_string());
        }
        if !r.strings("stmt.foreach.pair")?.is_empty() && lang.pair_mark.is_none() {
            return Err("stmt.foreach.pair needs syntax.map.pair".to_string());
        }
        if r.flag("ext.op.assign.compound")? {
            // Every binary operator followed by the assignment sign, unless
            // that spelling is already an operator (`<=`) or the operator
            // itself ends in the sign (`==`, whose `===` is another operator).
            let mut compound = HashMap::new();
            for (lex, op) in &lang.dyadic {
                for assign in &lang.assign_words {
                    let joined = format!("{lex}{assign}");
                    if !lex.ends_with(assign.as_str()) && !lang.dyadic.contains_key(&joined) && !lang.monadic.contains_key(&joined) {
                        compound.insert(joined, op.action.clone());
                    }
                }
            }
            lang.compound = compound;
        }
        if !lang.switch_words.is_empty() && (lang.case_words.is_empty() || lang.case_marks.is_empty()) {
            return Err("ext.stmt.switch needs ext.stmt.case and ext.stmt.case.mark".to_string());
        }
        if let Some(q) = lang.interpolating.iter().find(|q| !lang.quotes.contains(q)) {
            return Err(format!("ext.lexical.interpolating_quotes '{q}' is not among lexical.string_quotes"));
        }
        if !lang.interpolating.is_empty() && (lang.grouping.is_none() || lang.concat.is_none()) {
            return Err("ext.lexical.interpolating_quotes needs syntax.group and op.concat".to_string());
        }
        if lang.bare_calls && lang.calling.is_none() {
            return Err("ext.syntax.call.bare needs syntax.call".to_string());
        }
        lang.order_lexemes()?;
        Ok(lang)
    }

    /// Every lexeme is a symbol the scanner cuts on or a reserved word.
    fn order_lexemes(&mut self) -> Result<(), String> {
        let (unicode, prefix) = (self.unicode_names, self.sigil);
        let mut symbols: Vec<String> = Vec::new();
        let mut reserved: HashSet<String> = HashSet::new();
        let mut place = |lex: &str| {
            if name_like(lex, unicode, prefix) {
                reserved.insert(lex.to_string());
            } else if !symbols.iter().any(|s| s == lex) {
                symbols.push(lex.to_string());
            }
        };
        for lex in self.dyadic.keys().chain(self.monadic.keys()).chain(self.precedence.keys()).chain(self.compound.keys()) {
            place(lex);
        }
        if let Some((question, mark)) = &self.ternary {
            place(question);
            place(mark);
        }
        for mark in &self.long_quotes {
            place(mark);
        }
        for lex in &self.plus_words {
            place(lex);
        }
        for lex in &self.naming_words {
            place(lex);
        }
        if let Some(mark) = &self.pair_mark {
            place(mark);
        }
        for mark in [&self.bases_open, &self.bases_close, &self.member_mark, &self.scope_mark, &self.catch_between, &self.catch_tuple_open, &self.catch_tuple_close, &self.catch_group, &self.reference_mark, &self.otherwise_mark].into_iter().flatten() {
            place(mark);
        }
        for pair in [&self.grouping, &self.calling, &self.array_brackets, &self.map_brackets, &self.index_brackets].into_iter().flatten() {
            place(&pair.open);
            place(&pair.close);
            if let Some(sep) = &pair.between {
                place(sep);
            }
        }
        let mut lists: Vec<&Vec<String>> = vec![
            &self.comprehension_async, &self.comprehension_for, &self.comprehension_in, &self.comprehension_if, &self.array_spread, &self.map_spread, &self.block_intros, &self.assign_words, &self.stmt_ends, &self.argument_labels, &self.type_marks, &self.annotation_marks, &self.return_marks, &self.if_else_words, &self.lambda_words, &self.identity_not, &self.membership_words, &self.membership_not, &self.expression_assign, &self.ellipsis_words, &self.dup_words, &self.drop_words, &self.swap_words, &self.over_words, &self.rot_words, &self.eval_words, &self.quote_open, &self.long_quotes, &self.tuple_marks, &self.class_bases_open, &self.class_bases_close, &self.del_words, &self.nonlocal_words, &self.with_words, &self.with_as_words, &self.yield_words, &self.yield_from_words, &self.slice_ellipsis, &self.slice_marks, &self.quote_close, &self.increments, &self.decrements, &self.case_marks, &self.decorator_words, &self.carries_words, &self.carries_pairs, &self.keyword_only, &self.positional_only, &self.call_spread, &self.call_spread_pairs, &self.stmt_separators, &self.unpack_rest, &self.unpack_words,
        ];
        if self.blocks != Blocks::Indented {
            lists.push(&self.block_opens);
            lists.push(&self.block_closes);
        }
        for lex in lists.into_iter().flatten() {
            place(lex);
        }
        let keywords = [
            &self.let_words, &self.mutable_words, &self.if_words, &self.elif_words, &self.else_words, &self.while_words, &self.until_words, &self.for_words,
            &self.in_words, &self.return_words, &self.break_words, &self.continue_words, &self.function_words, &self.pass_words, &self.true_words,
            &self.false_words, &self.null_words, &self.c_for_words, &self.static_words, &self.global_words, &self.const_words,
            &self.switch_words, &self.case_words, &self.default_words, &self.foreach_words, &self.foreach_as_words,
            &self.class_words, &self.extends_words, &self.new_words, &self.modifier_words, &self.shared_words,
            &self.instanceof_words, &self.interface_words, &self.implements_words, &self.parent_words, &self.self_words, &self.try_words, &self.catch_words,
            &self.finally_words, &self.throw_words,
            &self.with_words, &self.with_as_words, &self.del_words, &self.nonlocal_words,
            &self.async_words, &self.await_words, &self.yield_words, &self.yield_from_words,
            &self.finally_words, &self.throw_words, &self.assert_words, &self.catch_as, &self.throw_from,
            &self.import_words, &self.import_from_words, &self.import_as_words,
        ];
        for word in keywords.into_iter().flatten() {
            if !name_like(word, unicode, prefix) {
                return Err(format!("keyword '{word}' must be shaped like an identifier"));
            }
            reserved.insert(word.clone());
        }
        if self.keywords_folded {
            reserved = reserved.into_iter().map(|w| w.to_lowercase()).collect();
        }
        symbols.sort_by_key(|s| std::cmp::Reverse(s.len()));
        self.symbols = symbols;
        self.keywords = reserved;
        Ok(())
    }

    pub fn spells(list: &[String], word: &str) -> bool {
        list.iter().any(|w| w == word)
    }

    pub fn ends_stmt(&self, lex: &str) -> bool {
        Lang::spells(&self.stmt_ends, lex) || Lang::spells(&self.stmt_separators, lex)
    }

    /// Whether a kind written before a parameter names a class: a word
    /// the language has a kind of its own for does not, nor does one it
    /// lists as naming none.
    pub fn names_a_class(&self, word: &str) -> bool {
        self.kind_of_word(word).is_none() && !Lang::spells(&self.loose_kinds, word)
    }

    /// The word this language names a value's kind by, the shorter one
    /// where it has one.
    pub fn kind_word(&self, v: &crate::value::Value) -> String {
        use crate::value::Sort;
        const IN_ORDER: [Sort; 7] =
            [Sort::Integer, Sort::Rational, Sort::Real, Sort::Text, Sort::Boolean, Sort::Array, Sort::Null];
        let Some(kind) = v.sort() else { return "value".to_string() };
        let at = IN_ORDER.iter().position(|k| *k == kind);
        match at.and_then(|i| self.brief_kinds.get(i)).filter(|word| *word != "-") {
            Some(word) => word.clone(),
            None => self.sort_bindings.iter().find(|(_, k)| *k == kind).map_or("value".to_string(), |(n, _)| n.clone()),
        }
    }

    /// The kind a word names, by the word the language asks a value's
    /// kind with or by the shorter word it complains with. Nothing where
    /// the word names no kind of the language's.
    pub fn kind_of_word(&self, word: &str) -> Option<crate::value::Sort> {
        use crate::value::Sort;
        const IN_ORDER: [Sort; 7] =
            [Sort::Integer, Sort::Rational, Sort::Real, Sort::Text, Sort::Boolean, Sort::Array, Sort::Null];
        if let Some((_, kind)) = self.sort_bindings.iter().find(|(n, _)| n == word) {
            return Some(*kind);
        }
        let at = self.brief_kinds.iter().position(|n| n != "-" && n == word)?;
        IN_ORDER.get(at).copied()
    }

    /// What a language says of a character it has no reading for. Where
    /// it names such characters by their number, it is named that way:
    /// a character there is no showing cannot be shown in a complaint
    /// either. Otherwise the kernel says it in its own plainer words.
    pub fn stopped_at_character(&self, c: char, line: usize, column: usize) -> String {
        match (&self.reading_unexpected, &self.reading_character) {
            (Some(opening), Some(named)) => format!("{opening} {named}{:02X}", c as u32),
            _ => format!("Unexpected character '{c}' at {line}:{column}"),
        }
    }

    /// What a language says of a key between the brackets of a name
    /// woven into text where the shorter writing does not take it.
    /// Nothing where the language says nothing, such a key then being
    /// left to stand as the letters it is written with.
    pub fn woven_index_amiss(&self) -> Option<String> {
        let (opening, said) = (self.reading_unexpected.as_ref()?, self.woven_index_words.as_ref()?);
        Some(format!("{opening} {said}"))
    }

    pub fn begins_name(&self, c: char) -> bool {
        if self.text_is_bytes {
            // Where text is bytes, a name is spelled in bytes too, and
            // every byte past the plain seven-bit ones may stand in one.
            return c == '_' || c.is_ascii_alphabetic() || (self.unicode_names && c >= '\u{80}');
        }
        c == '_' || if self.unicode_names { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
    }

    pub fn extends_name(&self, c: char) -> bool {
        if self.text_is_bytes {
            return c == '_' || c.is_ascii_alphanumeric() || (self.unicode_names && c >= '\u{80}');
        }
        c == '_' || if self.unicode_names { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
    }

    /// The bytes a piece of text stands for. Where text is bytes each
    /// character is one of them and is worth its own number; otherwise
    /// the bytes are the ones the letters are spelled with.
    pub fn bytes_of(&self, s: &str) -> Vec<u8> {
        match self.text_is_bytes {
            true => s.chars().map(|c| c as u32 as u8).collect(),
            false => s.as_bytes().to_vec(),
        }
    }

    /// The text a run of bytes stands for, which is the other way about.
    pub fn text_of(&self, bytes: &[u8]) -> String {
        match self.text_is_bytes {
            true => bytes.iter().map(|b| char::from(*b)).collect(),
            false => String::from_utf8_lossy(bytes).into_owned(),
        }
    }
}
