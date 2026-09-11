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
    pub complex_words: HashMap<String, Vec<String>>,
    pub method_keywords: HashMap<String, String>,
    pub value_methods: HashMap<String, String>,
    pub method_errors: HashMap<String, String>,
    pub fmt_text_format_zero_string: Vec<String>,
    pub fmt_text_format_zero_integer: Vec<String>,
    pub fmt_op_rem_format_infinity: Vec<String>,
    pub fmt_op_rem_format_nan: Vec<String>,
    pub open_decimal_point: bool,
    pub format_builtin: Vec<String>,
    pub format_method: Vec<String>,
    pub fmt_text_format_invalid: Vec<String>,
    pub fmt_text_format_unknown: Vec<String>,
    pub fmt_text_format_kinds: Vec<String>,
    pub fmt_text_format_unready: Vec<String>,
    pub fmt_text_format_precision_integer: Vec<String>,
    pub fmt_text_format_precision_missing: Vec<String>,
    pub fmt_text_format_sign_string: Vec<String>,
    pub fmt_text_format_alternate_string: Vec<String>,
    pub fmt_text_format_align_string: Vec<String>,
    pub fmt_text_format_sign_character: Vec<String>,
    pub fmt_text_format_alternate_character: Vec<String>,
    pub fmt_text_format_character: Vec<String>,
    pub fmt_text_format_spec_type: Vec<String>,
    pub fmt_text_format_numbered_auto: Vec<String>,
    pub fmt_text_format_numbered_manual: Vec<String>,
    pub fmt_text_format_index: Vec<String>,
    pub fmt_text_format_key: Vec<String>,
    pub fmt_text_format_brace_open: Vec<String>,
    pub fmt_text_format_brace_close: Vec<String>,
    pub fmt_text_format_conversion: Vec<String>,
    pub fmt_text_format_recursion: Vec<String>,
    pub fmt_op_rem_format_few: Vec<String>,
    pub fmt_op_rem_format_many: Vec<String>,
    pub fmt_op_rem_format_mapping: Vec<String>,
    pub fmt_op_rem_format_number: Vec<String>,
    pub fmt_op_rem_format_integer: Vec<String>,
    pub fmt_op_rem_format_real: Vec<String>,
    pub fmt_op_rem_format_character: Vec<String>,
    pub fmt_op_rem_format_star: Vec<String>,
    pub fmt_op_rem_format_incomplete: Vec<String>,
    pub fmt_op_rem_format_code: Vec<String>,

    pub text_words: HashMap<String, Vec<String>>,
    pub text_repeat: bool,
    pub text_negative_index: bool,
    pub class_details: HashMap<String, Vec<String>>,
    pub ident: String,
    pub extensions: Vec<String>,
    pub banner: String,
    pub continuation_amiss: Vec<String>,
    pub type_params_open: Vec<String>,
    pub type_params_close: Vec<String>,

    pub line_continuations: Vec<String>,
    pub bare_number_point: bool,
    pub separator_after_prefix: bool,
    pub whole_bits: bool,
    pub print_real_point: bool,
    pub with_as: Vec<String>,
    pub with_unready: Vec<String>,
    pub yield_from: Vec<String>,
    pub member_pipes: bool,
    pub tuple_unready: Vec<String>,
    pub byte_words: HashMap<String, Vec<String>>,
    pub bytes_unready: Vec<String>,
    pub format_unready: Vec<String>,
    pub identity_unready: Vec<String>,
    pub in_values: Vec<String>,
    pub in_not: Vec<String>,
    pub in_unready: Vec<String>,
    pub if_else: Vec<String>,
    pub assign_chain: bool,
    pub deferred_escapes: Vec<char>,
    pub imaginary_suffixes: Vec<String>,
    pub await_words: Vec<String>,
    pub async_words: Vec<String>,
    pub match_words: Vec<String>,
    pub match_cases: Vec<String>,
    pub type_alias_words: Vec<String>,
    pub type_parameters: bool,
    pub pipe_attribute: bool,
    pub line_comments: Vec<String>,
    pub ellipsis_words: Vec<String>,
    pub ellipsis_unready: Option<String>,
    /// The word for the value a method declines an operation with.
    pub unimplemented_words: Vec<String>,
    /// Whether a name the program binds stands in front of a builtin
    /// word spelled the same, when the name is called.
    pub shadow_builtins: bool,
    /// How many figures a whole number may be read from or written to
    /// text with, and the two pieces of words around the count when
    /// there are more; the words for a real past the numbers asked for
    /// as a whole number; whether a whole number rounded to a negative
    /// count of places rounds a half to the even neighbour.
    pub integer_digits: Option<usize>,
    pub digits_amiss: Vec<String>,
    pub to_int_infinity: Option<String>,
    pub to_int_nan: Option<String>,
    pub round_whole_even: bool,
    /// Words refusing a class built on the flag class, and words opening
    /// the complaint when a truth method answers with no flag.
    pub bool_base: Option<String>,
    /// The builtin kinds a class may stand on, the complaint when two
    /// of them are asked for in one class, and the method a mapping
    /// thing answers an absent key with.
    pub builtin_bases: Vec<String>,
    pub layout_amiss: Option<String>,
    pub missing_key: Option<String>,
    pub bool_result: Option<String>,
    /// The four pieces of words around the sign and the two kinds that
    /// stand in no order.
    pub order_unsupported: Vec<String>,
    pub lambda_words: Vec<String>,
    pub lambda_unready: Option<String>,
    pub expression_assign: Vec<String>,
    pub chained_calls: bool,
    pub chained_names: bool,
    pub with_words: Vec<String>,
    pub with_as_words: Vec<String>,
    pub tuple_marks: Vec<String>,
    pub matrix_words: Vec<String>,
    pub matrix_unready: Option<String>,
    pub block_comments: Vec<(String, String)>,
    pub quotes: Vec<char>,
    pub long_quotes: Vec<String>,
    pub adjacent_strings: bool,
    pub nonlocal_words: Vec<String>,
    pub nonlocal_unrun: Vec<String>,
    pub with_unrun: Vec<String>,
    pub async_unrun: Vec<String>,
    pub del_words: Vec<String>,
    pub del_unrun: String,
    pub loop_else: bool,
    pub continued_escapes: bool,
    pub yield_words: Vec<String>,
    pub yield_from_words: Vec<String>,
    pub raw_quotes: Vec<char>,
    pub raw_prefixes: Vec<char>,
    pub byte_prefixes: Vec<char>,
    pub plain_prefixes: Vec<char>,
    pub format_prefixes: Vec<char>,
    pub string_amiss: Option<String>,
    pub bytes_unavailable: Option<String>,
    pub format_unavailable: Option<String>,
    pub string_unready: Option<String>,
    pub range_zero_start: bool,
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
    pub imaginary_letters: Vec<char>,
    pub imaginary_unready: Option<String>,
    pub number_point_edge: bool,
    pub number_strict: bool,
    pub number_leading_zero: Option<String>,
    pub binary_amiss: Option<String>,
    pub binary_digit_amiss: Vec<String>,
    pub octal_amiss: Option<String>,
    pub octal_digit_amiss: Vec<String>,
    pub hex_amiss: Option<String>,

    pub prologue: Option<String>,
    pub point: Option<char>,
    pub bare_point: bool,
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
    /// A separator may follow a base prefix before its first digit.
    /// Bit operations keep every bit of a whole number.
    /// How many bits wide a whole number is, where a language says: a
    /// result that outgrows that width becomes a real instead.
    pub integer_bits: Option<usize>,
    /// How many bits wide a real is, where a language says its reals
    /// are binary numbers rather than exact ones, and how many
    /// significant digits one shows when simply written out.
    pub arithmetic_binary: bool,
    pub arithmetic_flags: bool,
    pub infinity_words: Vec<String>,
    pub nan_words: Vec<String>,
    pub point_open: bool,
    pub power_real: bool,
    pub power_overflow: Vec<String>,
    pub power_nonreal: Vec<String>,
    pub power_zero: Vec<String>,
    pub division_zero: Vec<String>,
    pub quotient_zero: Vec<String>,
    pub quotient_real_zero: Vec<String>,
    pub remainder_real_zero: Vec<String>,
    pub integer_text_detail: Vec<String>,
    pub bits_unbounded: bool,
    pub bits_integer: Vec<String>,
    pub bits_beyond: Vec<String>,
    pub real_bits: Option<usize>,
    pub shortest_reals: bool,
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
    pub whole_bits_amiss: Option<String>,
    pub whole_bits_large: Option<String>,
    pub matrix_unavailable: Option<String>,
    pub slice_marks: Vec<String>,
    pub slice_ellipsis: Vec<String>,
    pub slice_zero: Option<String>,
    pub slice_bounds: Option<String>,
    pub index_spread_unsupported: Option<String>,
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
    pub async_functions: Vec<String>,
    pub async_functions_unavailable: Vec<String>,
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
    pub core_words: HashMap<String, Vec<String>>,
    pub print_sep: Vec<String>,
    pub print_end: Vec<String>,
    pub print_file: Vec<String>,
    pub print_flush: Vec<String>,
    /// Where the printer sends its text when the definition routes it:
    /// a module, the member of it holding the stream, and the writer
    /// on that stream. Empty, the printer writes straight out.
    pub print_route: Vec<String>,
    /// Where the input builtin turns for its line: a module and the
    /// routine in it that reads.
    pub input_route: Vec<String>,
    /// The complaint for a stream builtin handed the wrong arguments,
    /// and the one for a stream that would not answer.
    pub stream_amiss: Vec<String>,
    pub stream_failed: Vec<String>,
    /// Whether the clock, handed a flag, answers in seconds and their
    /// parts: from the run's own start when the flag holds, else from
    /// the epoch.
    pub clock_parts: bool,
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
    pub exceptions: Vec<String>,
    pub exception_args: Option<String>,
    pub exception_cause: Option<String>,
    /// The member holding the exception that was being handled when
    /// this one was raised, and the flag that hides it; nothing where the
    /// language has no names for them.
    pub exception_context: Option<String>,
    pub exception_suppress: Option<String>,
    /// The name of the kind of a trace, where the language has one.
    pub exception_traceback: Option<String>,
    /// The fuller account of an exception: the method that adds a note
    /// and the list the notes stand in, the member holding a traceback
    /// and the method that would set one, the members naming an absent
    /// name and the object it was sought on, and the two members of an
    /// operating-system fault with the words around its number.
    pub note_method: Option<String>,
    pub notes_member: Option<String>,
    pub note_invalid: Option<String>,
    pub traceback_member: Option<String>,
    pub traceback_setter: Option<String>,
    pub absent_name_member: Option<String>,
    pub absent_object_member: Option<String>,
    pub os_members: Vec<String>,
    pub os_message: Vec<String>,
    /// An exception group: its message and its members, the three
    /// methods that part it, the words around its count when shown,
    /// and the complaint for members it may not take.
    pub group_message: Option<String>,
    pub group_members: Option<String>,
    pub group_split: Option<String>,
    pub group_subgroup: Option<String>,
    pub group_derive: Option<String>,
    pub group_summary: Vec<String>,
    pub group_invalid: Option<String>,
    pub class_name: Option<String>,
    pub exception_unready: Option<String>,
    pub fault_index: Option<String>,
    pub fault_key: Option<String>,
    /// Whether each statement of the program is marked with its line as
    /// the run goes, so that a call may be told where it was made from.
    pub marks_lines: bool,
    pub fault_name: Option<String>,
    pub fault_attribute: Option<String>,
    pub fault_stop: Option<String>,
    pub division_words: Option<String>,
    pub index_words: Option<String>,
    pub name_words: Vec<String>,
    pub attribute_words: Vec<String>,
    pub kind_words: Option<String>,
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
    pub bit_room: Option<String>,
    pub plus_non_number: Option<String>,
    pub left_shift_unready: Option<String>,
    pub bit_operands: Option<String>,
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
    pub class_bases_open: Vec<String>,
    pub class_bases_close: Vec<String>,
    pub special_stop: Vec<String>,
    pub special_declined: Vec<String>,
    pub special_unready: Vec<String>,
    pub class_special: Vec<String>,
    pub special_amiss: Vec<String>,
    /// Two pieces around the kind an index method wrongly answered with.
    pub index_answer_amiss: Vec<String>,
    /// Four pieces around the sign and the two kinds of a dyad neither
    /// operand's methods would take.
    pub operands_amiss: Vec<String>,
    /// Two pieces around the class name of a thing given a format
    /// specification it has no method for.
    pub format_spec_amiss: Vec<String>,
    /// The header keyword naming a metaclass, which no class form runs.
    pub metaclass_word: Vec<String>,
    pub class_unready: Vec<String>,
    pub yield_suspends: bool,
    pub yield_exhausted: Vec<String>,
    pub yield_send: Vec<String>,
    pub yield_close: Vec<String>,
    pub yield_throw: Vec<String>,
    pub yield_unstarted: Vec<String>,
    pub yield_busy: Vec<String>,
    pub yield_unsupported: Vec<String>,
    pub yield_throw_unavailable: Vec<String>,
    pub with_enter: Option<String>,
    pub with_leave: Option<String>,
    pub yield_unrun: Vec<String>,
    pub scope_unready: Vec<String>,
    pub global_words: Vec<String>,
    pub binding_unrun: String,
    pub import_values: bool,
    pub import_missing: Vec<String>,
    pub import_member_missing: Vec<String>,
    pub import_relative_unready: String,
    pub import_words: Vec<String>,
    pub import_from_words: Vec<String>,
    pub import_as_words: Vec<String>,
    pub math_floating: bool,
    pub module_helper_amiss: String,
    pub member_absent: Vec<String>,
    pub module_cache: Vec<String>,
    pub module_names: Vec<String>,
    /// The name a module keeps its opening documentation under
    /// (ext.system.module.doc), and the one it keeps the dictionary of
    /// builtin words under (ext.system.module.builtins).
    pub module_doc: Vec<String>,
    pub module_builtins: Vec<String>,
    /// The three ways text may be read ahead of time: as statements, as
    /// one expression, and as one statement shown as it runs
    /// (ext.builtin.compile.modes); the names compile gives its
    /// arguments (ext.builtin.compile.parameters); the class of the code
    /// value it hands back (ext.builtin.compile.kind).
    pub compile_modes: Vec<String>,
    pub compile_parameters: Vec<String>,
    pub compile_kind: Option<String>,
    /// The complaint for text that cannot be read
    /// (ext.builtin.source.syntax), and for a reading that cannot yet be
    /// honoured (ext.builtin.source.unready).
    pub source_syntax: Option<String>,
    pub source_unready: Option<String>,
    pub decorator_words: Vec<String>,
    pub decorator_amiss: Option<String>,
    pub const_words: Vec<String>,
    pub match_wildcards: Vec<String>,
    pub match_ors: Vec<String>,
    pub match_guards: Vec<String>,
    pub match_as: Vec<String>,
    pub match_unready: Vec<String>,
    pub match_invalid: Vec<String>,
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
    pub closes_over: bool,
    pub local_unbound: Vec<String>,
    pub free_unbound: Vec<String>,
    pub nonlocal_amiss: Vec<String>,
    pub nonlocal_module: Option<String>,
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
    pub imaginary_unrun: String,
    /// A sign that leaves its operand as it is.
    pub plus_words: Vec<String>,
    pub if_else_words: Vec<String>,
    pub lambda_enclosing: Option<String>,
    pub identity_not: Vec<String>,
    pub identity_unsupported: Option<String>,
    pub membership_words: Vec<String>,
    pub membership_not: Vec<String>,
    pub membership_unsupported: Option<String>,
    pub chained_comparisons: bool,
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
    pub tuple_separator: Option<String>,
    pub or_maps: bool,
    pub unordered_maps: bool,
    /// A map's keys stand for their worth: a flag is the number it
    /// counts as, and a whole number and the real it equals are one key.
    pub value_keys: bool,
    /// The words before and after the kind of a value that cannot key a
    /// map, and the words for a map that changed size under a walk.
    pub map_unhashable: Vec<String>,
    pub map_resized: Option<String>,
    pub print_separator: Option<String>,
    pub print_ending: Option<String>,
    pub print_option_type: Option<String>,
    pub map_argument_amiss: Option<String>,
    pub map_pair_amiss: Option<String>,
    pub set_words: HashMap<String, Vec<String>>,
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
    pub class_static: Vec<String>,
    pub class_method: Vec<String>,
    pub class_property: Vec<String>,
    pub property_setter: Vec<String>,
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
    /// The word for a routine a module answers absent names through.
    pub module_getattr: Option<String>,
    /// The word under which a class carries the names of its annotated
    /// members, in the order written.
    pub class_annotations: Vec<String>,
    /// The words for a class's own method that answers a call of the
    /// class itself, and for one that hands over what walking the class
    /// yields.
    pub class_called: Option<String>,
    /// Whether two texts are ordered letter by letter, by code point.
    pub text_ordered: bool,
    /// The words of the slice value: its three bounds' names, the complaints
    /// for the wrong count of bounds, a negative length and a bound written
    /// amiss, and the method a bound is asked for its whole number by.
    pub slice_parts: HashMap<String, String>,
    pub class_walked: Option<String>,
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
    /// Words for a thrown value that is no exception, where exceptions
    /// are furnished.
    pub throw_invalid: Option<String>,
    /// Words for a try mixing plain and grouped clauses, and for a
    /// generator whose body raises the exhaustion class.
    pub catch_amiss: Option<String>,
    pub yield_escaped: Option<String>,
    /// Words before and after the kind of a value that cannot manage a
    /// context.
    pub with_invalid: Vec<String>,
    /// The most calls that may be under way at once, and the words said
    /// by the one that would pass it.
    pub recursion_limit: Option<usize>,
    pub recursion_exceeded: Option<String>,
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
w builtin.to_real | w system.args | w system.memoization | w system.real_default_precision | s system.real.render
w system.entry | w system.kind.integer | w system.kind.rational | w system.kind.real
w system.kind.string | w system.kind.boolean | w system.kind.array | w system.kind.null
b system.flag.counts
";

/// The extension labels a definition may add beyond the core; a
/// missing one reads as empty (or off).
const EXT_LABELS: &str = "w ext.stmt.class.detail.call | w ext.stmt.class.detail.root | w ext.stmt.class.detail.mro | w ext.stmt.class.detail.order | w ext.stmt.class.detail.name | w ext.stmt.class.detail.qualified | w ext.stmt.class.detail.bases | w ext.stmt.class.detail.namespace | w ext.stmt.class.detail.kind | w ext.stmt.class.detail.allocate | w ext.stmt.class.detail.subclass | w ext.stmt.class.detail.slots | w ext.stmt.class.detail.set | w ext.stmt.class.detail.remove | w ext.stmt.class.detail.get | w ext.stmt.class.detail.getitem | w ext.stmt.class.detail.doc | w ext.stmt.class.detail.module | w ext.stmt.class.detail.defaults | w ext.stmt.class.detail.code | w ext.stmt.class.detail.argcount | w ext.stmt.class.detail.varnames | w ext.stmt.class.detail.receiver | w ext.stmt.class.detail.function | w ext.stmt.class.detail.locals | w ext.stmt.class.detail.main | w ext.stmt.class.detail.mro.amiss | w ext.stmt.class.detail.attribute.amiss | w ext.stmt.class.detail.unready | w ext.stmt.class.detail.descriptor.get | w ext.stmt.class.detail.descriptor.set | w ext.stmt.class.detail.descriptor.delete | w ext.stmt.class.detail.descriptor.name | w ext.stmt.class.detail.descriptor.foreign | w ext.stmt.class.detail.property.fget | w ext.stmt.class.detail.property.fset | w ext.stmt.class.detail.property.fdel | w ext.stmt.class.detail.property.getter | w ext.stmt.class.detail.property.deleter | w ext.stmt.class.detail.property.doc | w ext.stmt.class.detail.property.readonly | w ext.stmt.class.detail.property.unreadable | w ext.stmt.class.detail.property.unwritable | w ext.stmt.class.detail.property.undeletable | w ext.builtin.issubclass | w ext.builtin.callable | w ext.builtin.getattr | w ext.builtin.setattr | w ext.builtin.delattr | w ext.builtin.hasattr | w ext.builtin.vars | w ext.builtin.dir | w ext.builtin.staticmethod | w ext.builtin.classmethod | w ext.builtin.property | w ext.builtin.text.fault.protocol | b ext.op.index.text.negative | w ext.builtin.text.fault.index | w ext.builtin.text.splitlines | w ext.builtin.text.partition | w ext.builtin.text.rpartition | w ext.builtin.text.expandtabs | w ext.builtin.text.swapcase | w ext.builtin.text.casefold | w ext.builtin.text.capitalize | w ext.builtin.text.title | w ext.builtin.text.istitle | w ext.builtin.text.isidentifier | w ext.builtin.text.isprintable | w ext.builtin.text.isdecimal | w ext.builtin.text.isnumeric | w ext.builtin.text.isascii | w ext.builtin.text.removeprefix | w ext.builtin.text.removesuffix | w ext.builtin.text.format_map | w ext.builtin.text.maketrans | w ext.builtin.text.translate | w ext.builtin.text.encode | w ext.builtin.text.join | w ext.builtin.text.split | w ext.builtin.text.rsplit | w ext.builtin.text.strip | w ext.builtin.text.lstrip | w ext.builtin.text.rstrip | w ext.builtin.text.center | w ext.builtin.text.ljust | w ext.builtin.text.rjust | w ext.builtin.text.zfill | w ext.builtin.text.count | w ext.builtin.text.find | w ext.builtin.text.rfind | w ext.builtin.text.index | w ext.builtin.text.rindex | w ext.builtin.text.startswith | w ext.builtin.text.endswith | w ext.builtin.text.replace | w ext.builtin.text.upper | w ext.builtin.text.lower | w ext.builtin.text.length | w ext.builtin.text.repr | w ext.builtin.text.keyword.keepends | w ext.builtin.text.keyword.tabsize | w ext.builtin.text.keyword.maxsplit | w ext.builtin.text.keyword.sep | w ext.builtin.text.keyword.encoding | w ext.builtin.text.keyword.errors | w ext.builtin.text.fault.arguments | w ext.builtin.text.fault.receiver | w ext.builtin.text.fault.string | w ext.builtin.text.fault.integer | w ext.builtin.text.fault.separator | w ext.builtin.text.fault.fill | w ext.builtin.text.fault.missing | w ext.builtin.text.fault.encode | w ext.builtin.text.fault.mapping | w ext.builtin.text.fault.translation | w ext.builtin.text.fault.codepoint | w ext.builtin.text.fault.surrogate | w ext.builtin.text.fault.maketrans.length | w ext.builtin.text.fault.maketrans.key | w ext.builtin.text.fault.maketrans.type | w ext.builtin.text.fault.format | w ext.builtin.text.fault.format.positional | w ext.builtin.text.fault.format.brace | w ext.builtin.text.fault.join | w ext.builtin.text.fault.walk | w ext.builtin.text.fault.room | w ext.builtin.text.fault.key | w ext.builtin.text.complaint | b ext.builtin.text.repeat | w ext.builtin.bytes.signed | w ext.system.bytes.strict | w ext.builtin.bytes | w ext.builtin.bytearray | w ext.builtin.bytes.encode | w ext.builtin.bytes.decode | w ext.builtin.bytes.hex | w ext.builtin.bytes.fromhex | w ext.builtin.bytes.upper | w ext.builtin.bytes.lower | w ext.builtin.bytes.split | w ext.builtin.bytes.join | w ext.builtin.bytes.startswith | w ext.builtin.bytes.replace | w ext.builtin.bytes.strip | w ext.builtin.bytes.find | w ext.builtin.bytes.from_int | w ext.builtin.bytes.to_int | w ext.system.bytes.repr | w ext.system.bytes.type | w ext.system.bytes.encodings | w ext.system.bytes.order | w ext.system.bytes.unready | w ext.system.bytes.arguments | w ext.system.bytes.range | w ext.system.bytes.negative | w ext.system.bytes.index | w ext.system.bytes.immutable | w ext.system.bytes.unhashable | w ext.system.bytes.separator | w ext.system.bytes.hex | w ext.system.bytes.overflow | w ext.system.bytes.unsigned | w ext.system.bytes.bad_order | w ext.system.bytes.decode | w ext.system.bytes.encode | w ext.lexical.string.bytes.ascii | w ext.lexical.string.bytes.mixed | w ext.text.format.zero.integer | w ext.text.format.zero.string | w ext.op.rem.format.nan | w ext.op.rem.format.infinity | b ext.lexical.number.point.open | w ext.builtin.format | w ext.text.format | w ext.text.format.invalid | w ext.text.format.unknown | w ext.text.format.kinds | w ext.text.format.unready | w ext.text.format.precision.integer | w ext.text.format.precision.missing | w ext.text.format.sign.string | w ext.text.format.alternate.string | w ext.text.format.align.string | w ext.text.format.sign.character | w ext.text.format.alternate.character | w ext.text.format.character | w ext.text.format.spec.type | w ext.text.format.numbered.auto | w ext.text.format.numbered.manual | w ext.text.format.index | w ext.text.format.key | w ext.text.format.brace.open | w ext.text.format.brace.close | w ext.text.format.conversion | w ext.text.format.recursion | w ext.op.rem.format.few | w ext.op.rem.format.many | w ext.op.rem.format.mapping | w ext.op.rem.format.number | w ext.op.rem.format.integer | w ext.op.rem.format.real | w ext.op.rem.format.character | w ext.op.rem.format.star | w ext.op.rem.format.incomplete | w ext.op.rem.format.code | w ext.lexical.line_continuation | b ext.lexical.number.point.bare | b ext.lexical.number.separator.after_prefix | b ext.op.bit.whole | b ext.builtin.print.real_point | w ext.lexical.string.long | w ext.op.lambda | w ext.op.tuple | w ext.stmt.class.bases.open | w ext.stmt.class.bases.close | w ext.stmt.class.unready | w ext.stmt.class.builtin | w ext.stmt.class.layout | w ext.stmt.class.missing | w ext.stmt.del | w ext.stmt.nonlocal | w ext.stmt.nonlocal.unrun | w ext.stmt.with | w ext.stmt.with.as | w ext.stmt.yield | w ext.stmt.yield.from | w ext.stmt.yield.unrun | w ext.system.scope.unready | w ext.stmt.type_params.close | w ext.stmt.type_params.open | w ext.stmt.for.target.unready | w ext.op.identity.unready | w ext.op.identity.negated | w ext.op.identity | w ext.op.in.unready | w ext.op.in.negated | w ext.op.in | w ext.stmt.del.unrun | w ext.stmt.async.unready | w ext.op.await | w ext.stmt.async | w ext.stmt.with.unready | w ext.op.lambda.unready | b ext.op.member.pipes | w ext.op.tuple.unready | w ext.lexical.number.imaginary.unready | w ext.lexical.number.imaginary | w ext.lexical.string.amiss | w ext.lexical.line_continuation.amiss | w ext.lexical.string.prefix.raw | w ext.lexical.string.prefix.plain | w ext.lexical.string.prefix.bytes | w ext.lexical.string.prefix.format | b ext.lexical.string.adjacent | w ext.lexical.string.unready | b ext.lexical.escape.continued | b ext.stmt.loop.else | w ext.stmt.with.unrun | w ext.stmt.async.unrun | w ext.stmt.match | w ext.stmt.match.case | w ext.stmt.type_alias | b ext.stmt.type_parameters | b ext.op.pipe.attribute | b ext.stmt.yield.suspends | w ext.stmt.yield.exhausted | w ext.stmt.yield.send | w ext.stmt.yield.close | w ext.stmt.yield.throw | w ext.stmt.yield.unstarted | w ext.stmt.yield.busy | w ext.stmt.yield.unsupported | w ext.stmt.yield.throw.unavailable | w ext.builtin.next | w ext.builtin.iter | w ext.builtin.tuple | w ext.stmt.class.suite | w ext.stmt.class.suite.unsupported | w ext.lexical.string.raw_prefix | w ext.lexical.string.text_prefix | b ext.syntax.string.adjacent | w ext.op.identity.unsupported | b ext.op.comparison.chain | b ext.stmt.for.range.stop | w ext.stmt.with.group.open | w ext.stmt.with.group.close | w ext.stmt.with.unsupported | w ext.stmt.nonlocal.unsupported | w ext.stmt.delete | w ext.stmt.delete.unsupported | w ext.syntax.tuple.separator | w ext.syntax.tuple.unsupported | w ext.syntax.value.spread | w ext.syntax.value.spread.unsupported | w ext.op.index.spread.unsupported | w ext.op.conditional | b ext.stmt.function.short.bare | b ext.lexical.string.triple | w ext.stmt.class.special.unready | w ext.stmt.class.special.declined | w ext.stmt.class.special.stop | w ext.stmt.class.special | w ext.stmt.class.special.amiss | w ext.builtin.repr | w ext.builtin.hash | w ext.builtin.bool | w ext.builtin.sorted | w ext.builtin.isinstance | b ext.op.arithmetic.binary | b ext.op.arithmetic.flags | w ext.builtin.to_real.infinity | w ext.builtin.to_real.nan | b ext.lexical.number.point_open | b ext.op.pow.real_exponent | w ext.op.pow.overflow | w ext.op.pow.nonreal | w ext.op.pow.zero | w ext.op.div.zero | w ext.op.quot.zero | w ext.op.quot.real_zero | w ext.op.rem.real_zero | w ext.builtin.to_int.text.detail | b ext.op.bit.unbounded | w ext.op.bit.integer | w ext.op.bit.beyond | w ext.stmt.class.index.amiss | w ext.stmt.class.binary.amiss | w ext.stmt.class.format.amiss | w ext.stmt.class.metaclass
w ext.op.index.slice.ellipsis | w ext.op.index.slice | w ext.op.index.slice.zero | w ext.op.index.slice.bounds | w ext.op.index.slice.unsupported | w ext.op.index.slice.assign | w ext.op.index.slice.length | w ext.op.index.slice.detached
w ext.op.comprehension.async | w ext.op.comprehension.async.unavailable | w ext.op.comprehension.target.unavailable | w ext.builtin.sum.non_number | w ext.builtin.range.non_integer | w ext.builtin.range.zero_step | w ext.op.comprehension.for | w ext.op.comprehension.in | w ext.op.comprehension.if | b ext.syntax.set | w ext.syntax.array.spread | w ext.syntax.map.spread | w ext.syntax.collection.unwalkable | w ext.syntax.map.spread.unmapped | w ext.op.comprehension.unpack.amiss | b ext.builtin.range.value | w ext.builtin.sum | w ext.builtin.list | w ext.builtin.any | w ext.lexical.epilogue | w ext.system.args.list | w ext.system.args.count | w ext.lexical.prologue.echo | b ext.lexical.prologue.folded | w ext.builtin.echo | b ext.syntax.call.bare | w ext.op.increment | b ext.lexical.number.point.bare | b ext.lexical.number.separator.after_prefix | b ext.stmt.assign.names.chained | b ext.syntax.call.chained | w ext.op.lambda | w ext.op.lambda.unready | w ext.op.assign.expression | w ext.literal.ellipsis | w ext.literal.ellipsis.unready | w ext.stmt.with | w ext.stmt.with.as | w ext.stmt.with.unready | w ext.op.tuple | w ext.op.tuple.unready | w ext.stmt.class.unready | b ext.op.bit.whole | w ext.op.matrix | w ext.op.matrix.unready | w ext.lexical.line_continuation | w ext.op.contains | b ext.op.bit.or.maps | b ext.op.eq.maps.unordered | w ext.builtin.print.separator | w ext.builtin.print.end | w ext.builtin.print.option.type | w ext.builtin.map | w ext.builtin.map.arguments.amiss | w ext.builtin.map.pair.amiss | w ext.builtin.method.sort.key | w ext.builtin.method.sort.reverse | w ext.builtin.method.split.sep | w ext.builtin.method.split.maxsplit | w ext.builtin.method.upper | w ext.builtin.method.lower | w ext.builtin.method.strip | w ext.builtin.method.lstrip | w ext.builtin.method.rstrip | w ext.builtin.method.split | w ext.builtin.method.rsplit | w ext.builtin.method.join | w ext.builtin.method.replace | w ext.builtin.method.startswith | w ext.builtin.method.endswith | w ext.builtin.method.find | w ext.builtin.method.rfind | w ext.builtin.method.index | w ext.builtin.method.count | w ext.builtin.method.isdigit | w ext.builtin.method.isalpha | w ext.builtin.method.isalnum | w ext.builtin.method.isspace | w ext.builtin.method.islower | w ext.builtin.method.isupper | w ext.builtin.method.title | w ext.builtin.method.capitalize | w ext.builtin.method.center | w ext.builtin.method.ljust | w ext.builtin.method.rjust | w ext.builtin.method.zfill | w ext.builtin.method.format | w ext.builtin.method.encode | w ext.builtin.method.append | w ext.builtin.method.extend | w ext.builtin.method.insert | w ext.builtin.method.pop | w ext.builtin.method.remove | w ext.builtin.method.sort | w ext.builtin.method.reverse | w ext.builtin.method.copy | w ext.builtin.method.clear | w ext.builtin.method.get | w ext.builtin.method.keys | w ext.builtin.method.values | w ext.builtin.method.items | w ext.builtin.method.setdefault | w ext.builtin.method.update | w ext.builtin.method.bit_length | w ext.builtin.method.is_integer | w ext.builtin.method.hex | w ext.builtin.method.as_integer_ratio | w ext.builtin.method.error.unready | w ext.builtin.method.error.bytes | w ext.builtin.method.error.arguments | w ext.builtin.method.error.separator | w ext.builtin.method.error.substring | w ext.builtin.method.error.pop | w ext.builtin.method.error.index | w ext.builtin.method.error.remove | w ext.builtin.method.error.list_index | w ext.builtin.method.error.format | w ext.builtin.method.error.spec | w ext.builtin.method.error.unicode | w ext.builtin.method.error.attribute | w ext.builtin.method.error.key | w ext.builtin.method.error.missing | w ext.builtin.method.error.mixed | w ext.builtin.method.error.fill | w ext.builtin.method.error.hex | w ext.builtin.method.error.hex_overflow | w ext.builtin.sorted | w ext.builtin.isinstance | w ext.builtin.tuple | w ext.builtin.set | w ext.builtin.dict | w ext.builtin.reversed | w ext.builtin.enumerate | w ext.builtin.zip | w ext.builtin.filter | w ext.builtin.all | w ext.builtin.min | w ext.builtin.max | w ext.builtin.abs | w ext.builtin.round | w ext.builtin.divmod | w ext.builtin.pow | w ext.builtin.hex | w ext.builtin.oct | w ext.builtin.bin | w ext.builtin.repr | w ext.builtin.bool | w ext.builtin.callable | w ext.builtin.id | w ext.builtin.hash | w ext.builtin.iter | w ext.builtin.next | w ext.builtin.hasattr | w ext.builtin.getattr | w ext.builtin.setattr | w ext.builtin.delattr | w ext.builtin.vars | w ext.builtin.key | w ext.builtin.reverse | w ext.builtin.start | w ext.builtin.default | w ext.builtin.round.ndigits | w ext.builtin.round.number | w ext.builtin.pow.base | w ext.builtin.pow.exp | w ext.builtin.pow.mod | w ext.builtin.core.uniterable | w ext.builtin.core.uncallable | w ext.builtin.core.unhashable | w ext.builtin.core.unready | w ext.builtin.core.exhausted | w ext.builtin.core.isinstance.amiss | w ext.builtin.core.empty | w ext.builtin.core.arity | w ext.builtin.core.attribute | w ext.builtin.core.attribute.name | w ext.builtin.core.vars | w ext.builtin.core.zero | w ext.builtin.core.mod.zero | w ext.builtin.core.inverse | w ext.builtin.core.default.many | w ext.builtin.core.dict.pair | w ext.builtin.core.unindexable | w ext.builtin.core.immutable | w ext.builtin.core.power.zero | w ext.builtin.core.power.overflow | w ext.builtin.core.arity.one | w ext.builtin.core.arity.exact | w ext.builtin.core.integer | w ext.builtin.core.not_iterator | w ext.builtin.core.power.integer | w ext.builtin.core.dict.sequence | w ext.builtin.set.add | w ext.builtin.set.remove | w ext.builtin.set.discard | w ext.builtin.set.pop | w ext.builtin.set.clear | w ext.builtin.set.copy | w ext.builtin.set.update | w ext.builtin.set.union | w ext.builtin.set.intersection | w ext.builtin.set.difference | w ext.builtin.set.symmetric_difference | w ext.builtin.set.issubset | w ext.builtin.set.issuperset | w ext.builtin.set.isdisjoint | w ext.builtin.set.intersection_update | w ext.builtin.set.difference_update | w ext.builtin.set.symmetric_difference_update | w ext.builtin.set.sorted | w ext.builtin.set.method.unavailable | w ext.builtin.set.changed | w ext.builtin.set.unhashable | w ext.builtin.set.missing | w ext.builtin.set.empty | w ext.builtin.set.operands | w ext.builtin.set.arguments | w ext.builtin.set.unsupported | w ext.builtin.set.unsortable
w ext.op.decrement | w ext.lexical.interpolating_quotes | w ext.lexical.heredoc | b ext.lexical.escape.octal | b ext.system.text.bytes | w ext.lexical.prologue.brief | w ext.lexical.prologue.brief.setting | w ext.stmt.for.c | b ext.op.assign.compound
 | w ext.stmt.del.unrun | w ext.stmt.binding.unrun | b ext.stmt.loop.else | w ext.stmt.async | w ext.op.await | w ext.stmt.static | w ext.stmt.global | w ext.stmt.decorator | w ext.stmt.decorator.amiss | w ext.stmt.const | w ext.builtin.define | w ext.builtin.define.class_constant
b ext.stmt.import.value | w ext.stmt.import.missing | w ext.stmt.import.member.missing | w ext.stmt.import.relative.unready
w ext.builtin.program.namespace
w ext.builtin.member.get
w ext.builtin.member.set
w ext.builtin.instance
w ext.builtin.module.load
w ext.builtin.copy
w ext.syntax.map.resized | w ext.syntax.map.unhashable | b ext.syntax.map.value_keys | w ext.builtin.method.popitem | w ext.builtin.method.fromkeys | w ext.builtin.method.error.popitem
w ext.stmt.with.enter | w ext.stmt.with.leave
w ext.system.module.cache
w ext.builtin.module.helper.amiss | w ext.builtin.member.absent
b ext.builtin.math.floating
w ext.builtin.class.derive
w ext.builtin.call.outcome
w ext.stmt.import | w ext.stmt.import.from | w ext.stmt.import.as | w ext.system.module.name
w ext.stmt.static | w ext.stmt.global | w ext.stmt.decorator | w ext.stmt.decorator.amiss | w ext.stmt.const | w ext.builtin.define | w ext.builtin.define.class_constant
w ext.stmt.match | w ext.stmt.match.case | w ext.stmt.match.wildcard | w ext.stmt.match.or | w ext.stmt.match.guard | w ext.stmt.match.as | w ext.stmt.match.unready | w ext.stmt.match.invalid

w ext.builtin.var_dump | w ext.stmt.switch | w ext.stmt.case | w ext.stmt.default
w ext.stmt.case.mark | w ext.stmt.case.mark.instead | w ext.op.ternary | b ext.block.lone_statement | b ext.stmt.function.hoisted | b ext.stmt.function.outermost
w ext.system.request.amiss | w ext.system.request.amiss.boundary | w ext.system.request.amiss.boundary.wrong | w ext.system.request.amiss.part | w ext.system.request.amiss.body.large | w ext.system.request.body
w ext.lexical.number.imaginary | w ext.lexical.number.imaginary.unready | b ext.lexical.number.point_edge | b ext.lexical.number.separator.strict | w ext.lexical.number.amiss.leading_zero | w ext.lexical.number.amiss.binary | w ext.lexical.number.amiss.binary.digit | w ext.lexical.number.amiss.octal | w ext.lexical.number.amiss.octal.digit | w ext.lexical.number.amiss.hex | w ext.op.if_else | w ext.op.lambda.unsupported | w ext.op.lambda.enclosing | w ext.op.identical.negated | w ext.op.identical.unsupported | w ext.op.in | w ext.op.in.negated | w ext.op.in.unsupported | b ext.op.compare.chained | w ext.op.assign.expression | w ext.literal.ellipsis | b ext.op.rem.formats_text | w ext.op.rem.format.unsupported | w ext.op.rem.format.arguments | b ext.stmt.loop.else | b ext.lexical.string.adjacent | w ext.lexical.string.prefix.bytes | w ext.lexical.string.bytes.unready | w ext.op.lambda | w ext.lexical.number.imaginary.unrun | w ext.lexical.number.exponent | w ext.op.plus | b ext.stmt.break.levels | b ext.lexical.number.point.bare
w ext.builtin.array | b ext.op.index.append | b ext.stmt.for.collection | w ext.builtin.print_r
w ext.stmt.terminator | w ext.stmt.annotation | w ext.stmt.annotation.target.unready | w ext.stmt.annotation.amiss | w ext.stmt.function.returns | w ext.stmt.class | w ext.stmt.class.extends | w ext.stmt.class.new
w ext.stmt.class.this | w ext.stmt.class.constructor | w ext.stmt.class.destructor | w ext.stmt.class.reader | w ext.stmt.class.writer | w ext.stmt.class.caller
w ext.op.walk.class | w ext.op.walk.rewind | w ext.op.walk.more | w ext.op.walk.this | w ext.op.walk.key
w ext.op.walk.onward | w ext.op.walk.giver.class | w ext.op.walk.giver | w ext.op.walk.no_cell | w ext.op.walk.key.no_cell | b ext.op.walk.live | w ext.builtin.array.front | w ext.stmt.class.modifier | w ext.stmt.class.hidden | w ext.stmt.class.guarded | w ext.stmt.class.shared
w ext.op.member | w ext.op.scope | w ext.op.instanceof | w ext.stmt.class.parent
w ext.stmt.class.self | w ext.lexical.name_lead | w ext.stmt.assert | w ext.stmt.assert.kind | w ext.stmt.catch.invalid | w ext.stmt.catch.as | w ext.stmt.catch.tuple.open | w ext.stmt.catch.tuple.close | w ext.stmt.catch.group | w ext.stmt.catch.group.unsupported | b ext.stmt.try.else | w ext.stmt.throw.from | w ext.stmt.throw.empty | w ext.stmt.try | w ext.stmt.catch
w ext.stmt.finally | w ext.stmt.throw | w ext.stmt.catch.separator | w ext.op.reference
w ext.system.request.query | w ext.system.request.form | w ext.system.request.cookies | w ext.system.request.server
w ext.system.request.env | w ext.system.request.files | w ext.system.request.all | w ext.system.request.settings | b ext.op.index.absent | w ext.op.index.scalar | w ext.op.index.nothing | w ext.stmt.class.interface | w ext.stmt.class.implements | w ext.op.compare | w ext.builtin.unset | b ext.lexical.template | w ext.op.otherwise
w ext.lexical.line_continuation | b ext.lexical.number.separator.after_prefix | b ext.op.bit.whole | w ext.op.bit.whole.room | w ext.op.plus.non_number | w ext.op.bit.whole.amiss | w ext.op.bit.whole.large | w ext.op.matrix | w ext.op.matrix.unavailable | w ext.stmt.function.async | w ext.stmt.function.async.unavailable | w ext.op.bit.and | w ext.op.bit.or | w ext.op.bit.xor | w ext.op.bit.not | w ext.op.bit.left | w ext.op.bit.right | b ext.op.bit.shift.numbers | w ext.op.bit.left.unready | w ext.op.matrix.unready | w ext.op.bit.operands
w ext.op.identical | w ext.op.not_identical | b ext.system.kind.spelled
w ext.builtin.args.all | w ext.builtin.args.count | w ext.builtin.args.at
w ext.builtin.args.all.outside | w ext.builtin.args.count.outside | w ext.builtin.args.at.outside
w ext.builtin.args.at.below | w ext.builtin.args.at.beyond | b ext.op.assign.value | b ext.stmt.assign.chain | b ext.op.index.plain_keys
w ext.system.source.file | w ext.system.source.directory | w ext.system.source.line | w ext.system.runner
w ext.system.complaint.warning | w ext.system.complaint.notice | w ext.system.complaint.deprecated | w ext.system.complaint.fatal | w ext.system.complaint.reading
w ext.system.complaint.markup.setting | w ext.system.complaint.markup.kind | w ext.system.complaint.markup.place | w ext.system.complaint.markup.line | w ext.system.complaint.markup.reference
w ext.system.complaint.reference.setting | w ext.system.complaint.reference.page | w ext.system.complaint.reference.mark
w ext.builtin.include.demanded | w ext.builtin.include.demanded.missing
w ext.builtin.iter | w ext.builtin.next | w ext.builtin.repr | w ext.builtin.class.name | w ext.builtin.exceptions | w ext.builtin.exceptions.args | w ext.builtin.exceptions.cause | w ext.builtin.exceptions.unready | w ext.system.fault.attribute | w ext.system.fault.class.attribute | w ext.system.fault.class.index | w ext.system.fault.class.key | b ext.system.source.marked | w ext.system.fault.current | w ext.system.module.getattr | w ext.stmt.class.annotations | w ext.stmt.class.called | b ext.op.order.text | w ext.literal.unimplemented | b ext.syntax.names.shadow_builtins | w ext.builtin.method.bit_count | w ext.builtin.method.numerator | w ext.builtin.method.denominator | w ext.builtin.method.real | w ext.builtin.method.imag | w ext.builtin.method.__index__ | w ext.builtin.method.__truediv__ | w ext.builtin.method.fromhex | n ext.builtin.to_int.digits | w ext.builtin.to_int.digits.amiss | w ext.builtin.to_int.infinity | w ext.builtin.to_int.nan | b ext.builtin.round.whole.even | w ext.builtin.bool.base | w ext.builtin.bool.result | w ext.op.order.unsupported | w ext.builtin.slice | w ext.builtin.slice.start | w ext.builtin.slice.stop | w ext.builtin.slice.step | w ext.builtin.slice.arity | w ext.builtin.slice.length | w ext.op.index.integer | w ext.op.index.slice.amiss | w ext.builtin.method.indices | w ext.builtin.method.slice_hash | w ext.stmt.class.walked | w ext.system.fault.class.name | w ext.system.fault.class.stop | w ext.system.fault.division | w ext.system.fault.index | w ext.system.fault.kind | w ext.system.fault.name | w ext.system.fault.class | w ext.builtin.time_limit | w ext.system.kind.brief
w ext.builtin.file.read | w ext.builtin.file.write | w ext.builtin.file.exists | w ext.builtin.file.kind | w ext.builtin.host.info | w ext.builtin.file.remove | w ext.builtin.shell | w ext.builtin.wait | w ext.builtin.net.ask | w ext.builtin.run.begin | w ext.builtin.run.end
w ext.builtin.room.used | w ext.builtin.room.most | w ext.builtin.room.most.forget | w ext.builtin.room.limit
w ext.builtin.eval | w ext.builtin.include | w ext.builtin.include.once
w ext.builtin.print.redirect | w ext.builtin.input | w ext.builtin.input.reader | w ext.builtin.stream.write | w ext.builtin.stream.read | w ext.builtin.stream.amiss | w ext.builtin.stream.failed | w ext.builtin.output.hold | w ext.builtin.output.held | w ext.builtin.output.drop | w ext.builtin.output.depth | w ext.builtin.output.begun | w ext.builtin.at_end | w ext.builtin.complaint.handler | w ext.builtin.complaint.say | w ext.op.hush | w ext.builtin.isset | w ext.builtin.empty | w ext.stmt.do | b ext.op.index.makes | w ext.builtin.calls | w ext.system.kind.object | w ext.builtin.uncaught | w ext.builtin.classes | w ext.builtin.routines | w ext.builtin.spelled | w ext.builtin.class.beneath | w ext.builtin.math | w ext.builtin.class.methods | w ext.builtin.class.properties | b ext.builtin.write.operator | w ext.system.kind.loose | w ext.builtin.clock | b ext.builtin.clock.parts | w ext.stmt.class.trait | w ext.stmt.class.uses | w ext.stmt.class.uses.alias | b ext.syntax.call.bind_names | w ext.stmt.function.carries.pairs | w ext.stmt.function.keyword_only | w ext.stmt.function.positional_only | w ext.syntax.call.spread | w ext.syntax.call.spread.pairs | w ext.syntax.call.amiss | w ext.syntax.call.amiss.missing | w ext.syntax.call.amiss.unknown | w ext.syntax.call.amiss.duplicate | w ext.syntax.call.amiss.builtin | w ext.builtin.print.sep | w ext.builtin.print.end | w ext.builtin.print.file | w ext.builtin.print.flush | w ext.builtin.print.file.error | w ext.builtin.print.file.output | w ext.builtin.print.file.unready | w ext.builtin.print.sep.amiss | w ext.builtin.print.end.amiss | w ext.builtin.to_int.base | w ext.builtin.to_int.base.amiss | w ext.builtin.to_int.text.amiss | w ext.builtin.to_int.text.required | b ext.builtin.to_real.text | w ext.builtin.to_real.text.amiss | w ext.builtin.to_string.object | w ext.builtin.to_string.encoding | w ext.builtin.to_string.errors | w ext.builtin.to_string.unready | b ext.builtin.range.value | w ext.builtin.range.zero | w ext.builtin.range.integer | w ext.builtin.range.index | w ext.syntax.call.spread.amiss | w ext.syntax.call.spread.pairs.amiss | w ext.stmt.function.defaults.amiss | w ext.stmt.function.parameters.amiss | w ext.stmt.function.carries | w ext.stmt.function.short | w ext.builtin.exceptions.context | w ext.builtin.exceptions.suppress | w ext.builtin.exceptions.traceback | w ext.stmt.throw.invalid | w ext.stmt.with.invalid | n ext.system.recursion.limit | w ext.system.recursion.exceeded | w ext.builtin.exceptions.note | w ext.builtin.exceptions.notes | w ext.builtin.exceptions.note.invalid | w ext.builtin.exceptions.traceback.member | w ext.builtin.exceptions.traceback.with | w ext.builtin.exceptions.name | w ext.builtin.exceptions.object | w ext.builtin.exceptions.os | w ext.builtin.exceptions.os.message | w ext.builtin.exceptions.group.message | w ext.builtin.exceptions.group.members | w ext.builtin.exceptions.group.split | w ext.builtin.exceptions.group.subgroup | w ext.builtin.exceptions.group.derive | w ext.builtin.exceptions.group.summary | w ext.builtin.exceptions.group.invalid | w ext.system.fault.held | w ext.stmt.catch.amiss | w ext.stmt.yield.escaped
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

w ext.lexical.line_continuation | w ext.lexical.string.bytes.unavailable | w ext.lexical.string.format.unavailable | w ext.lexical.string.long | w ext.lexical.string.prefix.raw | w ext.lexical.string.prefix.bytes | w ext.lexical.string.prefix.plain | w ext.lexical.string.prefix.format | b ext.lexical.string.adjacent | w ext.lexical.string.amiss | n ext.lexical.escape.byte.digits | n ext.lexical.escape.codepoint.digits | w ext.lexical.escape.codepoint.wide | n ext.lexical.escape.codepoint.wide.digits | w ext.lexical.escape.named | w ext.lexical.escape.unavailable | w ext.lexical.string.value.unready | b ext.builtin.range.zero_start | w ext.lexical.string.unready
b ext.lexical.escape.continued | w ext.lexical.escape.controls | w ext.lexical.escape.codepoint | w ext.lexical.escape.codepoint.open | w ext.lexical.escape.codepoint.close
w ext.lexical.escape.codepoint.amiss | w ext.lexical.escape.codepoint.beyond | w ext.lexical.number.amiss
w ext.lexical.escape.byte | w ext.lexical.interpolating.index.amiss | w ext.builtin.eval.place
w ext.system.reading.unexpected | w ext.system.reading.unexpected.character | w ext.system.fault.class.reading
w ext.system.reading.unclosed | w ext.system.reading.unclosed.line | w ext.system.reading.unclosed.mismatch | w ext.system.reading.unmatched
w ext.lexical.number.binary_prefix | w ext.lexical.number.octal_prefix | b ext.lexical.number.octal_lead | w ext.lexical.number.separator | b ext.lexical.number.separator.after_prefix | b ext.op.bit.whole
n ext.system.integer.bits | n ext.system.real.bits | n ext.system.real.digits
w ext.system.real.figures | w ext.system.real.figures.shown
w ext.stmt.class.bases.open | w ext.stmt.class.bases.close | b ext.stmt.class.this.explicit | b ext.op.member.pipes | w ext.stmt.class.unready | b ext.stmt.function.own_names | b ext.stmt.static.read_in | w ext.stmt.with.unready | w ext.op.tuple.unready | w ext.lexical.string.prefix.bytes.unready | w ext.lexical.string.prefix.format.unready | b ext.stmt.assign.chain | w ext.lexical.escape.deferred | b ext.stmt.function.closes_over | w ext.stmt.function.local.unbound | w ext.stmt.function.free.unbound | w ext.stmt.nonlocal.amiss | w ext.stmt.nonlocal.module | w ext.stmt.class.static | w ext.stmt.class.classmethod | w ext.stmt.class.property | w ext.stmt.class.property.setter
 | w ext.builtin.complex | w ext.builtin.complex.real | w ext.builtin.complex.imag | w ext.builtin.method.conjugate | w ext.builtin.complex.invalid | w ext.builtin.complex.integer | w ext.builtin.complex.order | w ext.builtin.complex.floor | w ext.builtin.complex.zero | w ext.builtin.complex.power.zero | w ext.builtin.complex.unready
w ext.builtin.core.unsized | w ext.builtin.core.dict.changed | w ext.builtin.zip.strict | w ext.builtin.zip.short | w ext.builtin.zip.long
w ext.builtin.globals | w ext.builtin.locals | w ext.builtin.exec | w ext.builtin.compile | w ext.builtin.compile.modes | w ext.builtin.compile.parameters | w ext.builtin.compile.kind | w ext.builtin.source.syntax | w ext.builtin.source.unready | w ext.builtin.import | w ext.system.module.doc | w ext.system.module.builtins ";

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
    /// Text of a number with the digit separators taken out, or nothing
    /// where one of them stands anywhere but between two figures. Text
    /// with no separator in it comes back as it was.
    pub fn number_separator_between_digits(&self, text: &str) -> Option<String> {
        if self.digit_separators.is_empty() || !text.contains(|c| self.digit_separators.contains(&c)) {
            return Some(text.to_string());
        }
        let letters: Vec<char> = text.chars().collect();
        let mut joined = String::new();
        for (at, &letter) in letters.iter().enumerate() {
            if !self.digit_separators.contains(&letter) {
                joined.push(letter);
                continue;
            }
            let before = at.checked_sub(1).and_then(|i| letters.get(i)).map_or(false, char::is_ascii_digit);
            let after = letters.get(at + 1).map_or(false, char::is_ascii_digit);
            if !(before && after) { return None; }
        }
        Some(joined)
    }

    /// Whether the definition has slice values of its own: a builtin
    /// that makes one, and so keys of several places and a slice's
    /// complaints told under the classes the definition gave them.
    pub fn slice_values(&self) -> bool {
        self.slice_parts.get("ext.builtin.slice").map_or(false, |word| !word.is_empty())
    }

    /// Whether these words are a complaint of the value protocol, already
    /// told under its class: a class that may not be built on, a truth
    /// method's wrong answer, two values in no order.
    pub fn protocol_named(&self, said: &str) -> bool {
        let opens_with = |words: &[String]| words.first().map_or(false, |opening| !opening.is_empty() && said.starts_with(opening.as_str()));
        self.bool_base.as_deref() == Some(said)
            || self.layout_amiss.as_deref() == Some(said)
            || self.catch_amiss.as_deref() == Some(said)
            || self.bool_result.as_deref().map_or(false, |opening| !opening.is_empty() && said.starts_with(opening))
            || self.fault_shift.as_deref() == Some(said)
            || self.to_int_infinity.as_deref() == Some(said) || self.to_int_nan.as_deref() == Some(said)
            || opens_with(&self.digits_amiss) || opens_with(&self.integer_text_detail)
            || self.core_between("core.not_iterator", said) || self.core_between("core.uniterable", said) || self.core_between("core.unsized", said)
            || self.core_words.get("core.dict.changed").and_then(|words| words.first()).map_or(false, |whole| whole == said)
            || self.uneven_zip_named("zip.short", said) || self.uneven_zip_named("zip.long", said)
            || self.between_pieces(&self.index_answer_amiss, said) || self.between_pieces(&self.format_spec_amiss, said)
            || match self.order_unsupported.as_slice() {
                [before, between, and, after] => !before.is_empty() && said.starts_with(before.as_str()) && said.contains(between.as_str()) && said.contains(and.as_str()) && said.ends_with(after.as_str()),
                _ => false,
            }
            || match self.operands_amiss.as_slice() {
                [before, between, and, after] => !before.is_empty() && said.starts_with(before.as_str()) && said.contains(between.as_str()) && said.contains(and.as_str()) && said.ends_with(after.as_str()),
                _ => false,
            }
    }

    /// Whether the words open and close with the two pieces of a
    /// protocol complaint that stands either side of a name.
    fn between_pieces(&self, pieces: &[String], said: &str) -> bool {
        matches!(pieces, [before, after] if !before.is_empty() && said.starts_with(before.as_str()) && said.ends_with(after.as_str()))
    }

    /// Whether the protocol list reaches the in-place operations, so
    /// that a compound write may ask the place it lands on first.
    pub fn in_place_methods(&self) -> bool { self.class_special.len() > 59 }

    /// Whether the words open and close with the two pieces of a core
    /// word that stands either side of a kind.
    fn core_between(&self, label: &str, said: &str) -> bool {
        matches!(self.core_words.get(label).map(Vec::as_slice), Some([before, after]) if !before.is_empty() && said.starts_with(before.as_str()) && said.ends_with(after.as_str()))
    }

    /// Whether the words are zip's complaint of sources of unequal
    /// length, which opens with its first piece and closes with either
    /// of the other two, the last followed by a number.
    fn uneven_zip_named(&self, label: &str, said: &str) -> bool {
        matches!(self.core_words.get(label).map(Vec::as_slice), Some([opening, one, many]) if !opening.is_empty() && said.starts_with(opening.as_str()) && (said.ends_with(one.as_str()) || said.contains(many.as_str())))
    }

    /// Whether these words are a slice's own complaint, already told
    /// under its class where the definition has slice values.
    pub fn slice_named(&self, said: &str) -> bool {
        if !self.slice_values() { return false; }
        [&self.slice_zero, &self.slice_bounds, &self.slice_assign].iter().any(|words| words.as_deref() == Some(said))
            || ["ext.builtin.slice.arity", "ext.builtin.slice.length", "ext.op.index.slice.amiss"].iter()
                .any(|label| self.slice_parts.get(*label).map_or(false, |words| !words.is_empty() && words == said))
            || self.slice_length.first().map_or(false, |opening| !opening.is_empty() && said.starts_with(opening.as_str()))
    }

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
            ("ext.op.contains", Action::Contains), ("op.add", Action::Add), ("op.sub", Action::Sub), ("op.mul", Action::Mul), ("op.div", div),
            ("op.quot", Action::IntDiv), ("op.rem", Action::Mod), ("op.pow", Action::Power), ("op.eq", Action::Eq),
            ("op.ne", Action::Ne), ("op.lt", Action::Lt), ("op.le", Action::Le), ("op.gt", Action::Gt), ("op.ge", Action::Ge),
            ("op.and", Action::And), ("op.or", Action::Or), ("op.concat", Action::Join), ("ext.op.compare", Action::Rank),
            ("ext.op.matrix", Action::Matrix), ("ext.op.bit.and", Action::BitBoth), ("ext.op.bit.or", Action::BitEither), ("ext.op.bit.xor", Action::BitOne),
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
            ("ext.builtin.complex", Builtin::Complex),
            ("ext.builtin.method.conjugate", Builtin::ValueMethod),
            ("ext.builtin.method.bit_count", Builtin::ValueMethod), ("ext.builtin.method.numerator", Builtin::ValueMethod), ("ext.builtin.method.denominator", Builtin::ValueMethod), ("ext.builtin.method.real", Builtin::ValueMethod), ("ext.builtin.method.imag", Builtin::ValueMethod), ("ext.builtin.method.__index__", Builtin::ValueMethod), ("ext.builtin.method.__truediv__", Builtin::ValueMethod), ("ext.builtin.method.fromhex", Builtin::ValueMethod), ("ext.builtin.method.indices", Builtin::ValueMethod), ("ext.builtin.method.slice_hash", Builtin::ValueMethod), ("ext.builtin.method.popitem", Builtin::ValueMethod), ("ext.builtin.method.fromkeys", Builtin::ValueMethod),
            ("ext.builtin.isinstance", Builtin::InstanceOf),
            ("ext.builtin.tuple", Builtin::Tuple),
            ("ext.builtin.set", Builtin::Set),
            ("ext.builtin.dict", Builtin::Dict),
            ("ext.builtin.sorted", Builtin::Sorted),
            ("ext.builtin.reversed", Builtin::Reversed),
            ("ext.builtin.enumerate", Builtin::Enumerate),
            ("ext.builtin.zip", Builtin::Zip),
            ("ext.builtin.map", Builtin::Map),
            ("ext.builtin.filter", Builtin::Filter),
            ("ext.builtin.all", Builtin::All),
            ("ext.builtin.min", Builtin::Minimum),
            ("ext.builtin.max", Builtin::Maximum),
            ("ext.builtin.abs", Builtin::Absolute),
            ("ext.builtin.round", Builtin::Round),
            ("ext.builtin.divmod", Builtin::Divmod),
            ("ext.builtin.pow", Builtin::Power),
            ("ext.builtin.hex", Builtin::Hex),
            ("ext.builtin.oct", Builtin::Oct),
            ("ext.builtin.bin", Builtin::Bin),
            ("ext.builtin.repr", Builtin::Repr),
            ("ext.builtin.bool", Builtin::Bool),
            ("ext.builtin.callable", Builtin::Callable),
            ("ext.builtin.id", Builtin::Identity),
            ("ext.builtin.hash", Builtin::Hash),
            ("ext.builtin.iter", Builtin::Iter),
            ("ext.builtin.next", Builtin::Next),
            ("ext.builtin.hasattr", Builtin::HasAttr),
            ("ext.builtin.getattr", Builtin::GetAttr),
            ("ext.builtin.setattr", Builtin::SetAttr),
            ("ext.builtin.delattr", Builtin::DelAttr),
            ("ext.builtin.vars", Builtin::Vars),
            ("ext.builtin.globals", Builtin::OuterNames),
            ("ext.builtin.locals", Builtin::NearNames),
            ("ext.builtin.exec", Builtin::RunText),
            ("ext.builtin.compile", Builtin::ReadyText),
            ("ext.builtin.import", Builtin::Summon),
            ("ext.builtin.sum", Builtin::Sum),
            ("ext.builtin.list", Builtin::List),
            ("ext.builtin.any", Builtin::Any),
            ("ext.builtin.method.upper", Builtin::ValueMethod),
            ("ext.builtin.method.lower", Builtin::ValueMethod),
            ("ext.builtin.method.strip", Builtin::ValueMethod),
            ("ext.builtin.method.lstrip", Builtin::ValueMethod),
            ("ext.builtin.method.rstrip", Builtin::ValueMethod),
            ("ext.builtin.method.split", Builtin::ValueMethod),
            ("ext.builtin.method.rsplit", Builtin::ValueMethod),
            ("ext.builtin.method.join", Builtin::ValueMethod),
            ("ext.builtin.method.replace", Builtin::ValueMethod),
            ("ext.builtin.method.startswith", Builtin::ValueMethod),
            ("ext.builtin.method.endswith", Builtin::ValueMethod),
            ("ext.builtin.method.find", Builtin::ValueMethod),
            ("ext.builtin.method.rfind", Builtin::ValueMethod),
            ("ext.builtin.method.index", Builtin::ValueMethod),
            ("ext.builtin.method.count", Builtin::ValueMethod),
            ("ext.builtin.method.isdigit", Builtin::ValueMethod),
            ("ext.builtin.method.isalpha", Builtin::ValueMethod),
            ("ext.builtin.method.isalnum", Builtin::ValueMethod),
            ("ext.builtin.method.isspace", Builtin::ValueMethod),
            ("ext.builtin.method.islower", Builtin::ValueMethod),
            ("ext.builtin.method.isupper", Builtin::ValueMethod),
            ("ext.builtin.method.title", Builtin::ValueMethod),
            ("ext.builtin.method.capitalize", Builtin::ValueMethod),
            ("ext.builtin.method.center", Builtin::ValueMethod),
            ("ext.builtin.method.ljust", Builtin::ValueMethod),
            ("ext.builtin.method.rjust", Builtin::ValueMethod),
            ("ext.builtin.method.zfill", Builtin::ValueMethod),
            ("ext.builtin.method.format", Builtin::ValueMethod),
            ("ext.builtin.method.encode", Builtin::ValueMethod),
            ("ext.builtin.method.append", Builtin::ValueMethod),
            ("ext.builtin.method.extend", Builtin::ValueMethod),
            ("ext.builtin.method.insert", Builtin::ValueMethod),
            ("ext.builtin.method.pop", Builtin::ValueMethod),
            ("ext.builtin.method.remove", Builtin::ValueMethod),
            ("ext.builtin.method.sort", Builtin::ValueMethod),
            ("ext.builtin.method.reverse", Builtin::ValueMethod),
            ("ext.builtin.method.copy", Builtin::ValueMethod),
            ("ext.builtin.method.clear", Builtin::ValueMethod),
            ("ext.builtin.method.get", Builtin::ValueMethod),
            ("ext.builtin.method.keys", Builtin::ValueMethod),
            ("ext.builtin.method.values", Builtin::ValueMethod),
            ("ext.builtin.method.items", Builtin::ValueMethod),
            ("ext.builtin.method.setdefault", Builtin::ValueMethod),
            ("ext.builtin.method.update", Builtin::ValueMethod),
            ("ext.builtin.method.bit_length", Builtin::ValueMethod),
            ("ext.builtin.method.is_integer", Builtin::ValueMethod),
            ("ext.builtin.method.hex", Builtin::ValueMethod),
            ("ext.builtin.method.as_integer_ratio", Builtin::ValueMethod),
            ("ext.builtin.set.add", Builtin::SetAdd),
            ("ext.builtin.set.remove", Builtin::SetRemove),
            ("ext.builtin.set.discard", Builtin::SetDiscard),
            ("ext.builtin.set.pop", Builtin::SetPop),
            ("ext.builtin.set.clear", Builtin::SetClear),
            ("ext.builtin.set.copy", Builtin::SetCopy),
            ("ext.builtin.set.update", Builtin::SetUpdate),
            ("ext.builtin.set.union", Builtin::SetUnion),
            ("ext.builtin.set.intersection", Builtin::SetIntersection),
            ("ext.builtin.set.difference", Builtin::SetDifference),
            ("ext.builtin.set.symmetric_difference", Builtin::SetSymmetric),
            ("ext.builtin.set.issubset", Builtin::SetSubset),
            ("ext.builtin.set.issuperset", Builtin::SetSuperset),
            ("ext.builtin.set.isdisjoint", Builtin::SetDisjoint),
            ("ext.builtin.set.intersection_update", Builtin::SetMeetUpdate),
            ("ext.builtin.set.difference_update", Builtin::SetLessUpdate),
            ("ext.builtin.set.symmetric_difference_update", Builtin::SetXorUpdate),
            ("ext.builtin.set.sorted", Builtin::Sorted),
            ("ext.builtin.format", Builtin::Format),
            ("ext.builtin.bytes", Builtin::Bytes(0)),
            ("ext.builtin.bytearray", Builtin::Bytes(1)),
            ("ext.builtin.bytes.encode", Builtin::Bytes(2)),
            ("ext.builtin.bytes.decode", Builtin::Bytes(3)),
            ("ext.builtin.bytes.hex", Builtin::Bytes(4)),
            ("ext.builtin.bytes.fromhex", Builtin::Bytes(5)),
            ("ext.builtin.bytes.upper", Builtin::Bytes(6)),
            ("ext.builtin.bytes.lower", Builtin::Bytes(7)),
            ("ext.builtin.bytes.split", Builtin::Bytes(8)),
            ("ext.builtin.bytes.join", Builtin::Bytes(9)),
            ("ext.builtin.bytes.startswith", Builtin::Bytes(10)),
            ("ext.builtin.bytes.replace", Builtin::Bytes(11)),
            ("ext.builtin.bytes.strip", Builtin::Bytes(12)),
            ("ext.builtin.bytes.find", Builtin::Bytes(13)),
            ("ext.builtin.bytes.from_int", Builtin::Bytes(14)),
            ("ext.builtin.bytes.to_int", Builtin::Bytes(15)),
            ("ext.builtin.text.splitlines", Builtin::Text(crate::strings::TextOp::Splitlines)),
            ("ext.builtin.text.partition", Builtin::Text(crate::strings::TextOp::Partition)),
            ("ext.builtin.text.rpartition", Builtin::Text(crate::strings::TextOp::Rpartition)),
            ("ext.builtin.text.expandtabs", Builtin::Text(crate::strings::TextOp::Expandtabs)),
            ("ext.builtin.text.swapcase", Builtin::Text(crate::strings::TextOp::Swapcase)),
            ("ext.builtin.text.casefold", Builtin::Text(crate::strings::TextOp::Casefold)),
            ("ext.builtin.text.capitalize", Builtin::Text(crate::strings::TextOp::Capitalize)),
            ("ext.builtin.text.title", Builtin::Text(crate::strings::TextOp::Title)),
            ("ext.builtin.text.istitle", Builtin::Text(crate::strings::TextOp::Istitle)),
            ("ext.builtin.text.isidentifier", Builtin::Text(crate::strings::TextOp::Isidentifier)),
            ("ext.builtin.text.isprintable", Builtin::Text(crate::strings::TextOp::Isprintable)),
            ("ext.builtin.text.isdecimal", Builtin::Text(crate::strings::TextOp::Isdecimal)),
            ("ext.builtin.text.isnumeric", Builtin::Text(crate::strings::TextOp::Isnumeric)),
            ("ext.builtin.text.isascii", Builtin::Text(crate::strings::TextOp::Isascii)),
            ("ext.builtin.text.removeprefix", Builtin::Text(crate::strings::TextOp::Removeprefix)),
            ("ext.builtin.text.removesuffix", Builtin::Text(crate::strings::TextOp::Removesuffix)),
            ("ext.builtin.text.format_map", Builtin::Text(crate::strings::TextOp::FormatMap)),
            ("ext.builtin.text.maketrans", Builtin::Text(crate::strings::TextOp::Maketrans)),
            ("ext.builtin.text.translate", Builtin::Text(crate::strings::TextOp::Translate)),
            ("ext.builtin.text.encode", Builtin::Text(crate::strings::TextOp::Encode)),
            ("ext.builtin.text.join", Builtin::Text(crate::strings::TextOp::Join)),
            ("ext.builtin.text.split", Builtin::Text(crate::strings::TextOp::Split)),
            ("ext.builtin.text.rsplit", Builtin::Text(crate::strings::TextOp::Rsplit)),
            ("ext.builtin.text.strip", Builtin::Text(crate::strings::TextOp::Strip)),
            ("ext.builtin.text.lstrip", Builtin::Text(crate::strings::TextOp::Lstrip)),
            ("ext.builtin.text.rstrip", Builtin::Text(crate::strings::TextOp::Rstrip)),
            ("ext.builtin.text.center", Builtin::Text(crate::strings::TextOp::Center)),
            ("ext.builtin.text.ljust", Builtin::Text(crate::strings::TextOp::Ljust)),
            ("ext.builtin.text.rjust", Builtin::Text(crate::strings::TextOp::Rjust)),
            ("ext.builtin.text.zfill", Builtin::Text(crate::strings::TextOp::Zfill)),
            ("ext.builtin.text.count", Builtin::Text(crate::strings::TextOp::Count)),
            ("ext.builtin.text.find", Builtin::Text(crate::strings::TextOp::Find)),
            ("ext.builtin.text.rfind", Builtin::Text(crate::strings::TextOp::Rfind)),
            ("ext.builtin.text.index", Builtin::Text(crate::strings::TextOp::Index)),
            ("ext.builtin.text.rindex", Builtin::Text(crate::strings::TextOp::Rindex)),
            ("ext.builtin.text.startswith", Builtin::Text(crate::strings::TextOp::Startswith)),
            ("ext.builtin.text.endswith", Builtin::Text(crate::strings::TextOp::Endswith)),
            ("ext.builtin.text.replace", Builtin::Text(crate::strings::TextOp::Replace)),
            ("ext.builtin.text.upper", Builtin::Text(crate::strings::TextOp::Upper)),
            ("ext.builtin.text.lower", Builtin::Text(crate::strings::TextOp::Lower)),
            ("ext.builtin.text.length", Builtin::Text(crate::strings::TextOp::Length)),
            ("ext.builtin.text.repr", Builtin::Text(crate::strings::TextOp::Repr)),
            ("builtin.emit", Builtin::Echo), ("builtin.print", Builtin::Say), ("builtin.write", Builtin::Out),
            ("builtin.len", Builtin::Length), ("builtin.char_at", Builtin::CharAtIndex), ("builtin.ord", Builtin::CodeOf),
             ("ext.builtin.issubclass", Builtin::ClassTool(1)),       ("ext.builtin.dir", Builtin::ClassTool(8)), ("ext.builtin.staticmethod", Builtin::ClassTool(9)), ("ext.builtin.classmethod", Builtin::ClassTool(10)), ("ext.builtin.property", Builtin::ClassTool(11)),
            ("builtin.chr", Builtin::CharOf), ("builtin.typeof", Builtin::SortOf), ("builtin.error", Builtin::Raise),
            ("builtin.extern", Builtin::External), ("builtin.range", Builtin::Span), ("builtin.real", Builtin::MakeReal),
            ("builtin.precision", Builtin::Places),    ("builtin.to_string", Builtin::ToText),
            ("builtin.to_int", Builtin::ToInt), ("builtin.to_real", Builtin::AsReal), ("builtin.num", Builtin::Numer),
            ("builtin.den", Builtin::Denom), ("builtin.push", Builtin::Append), ("builtin.get", Builtin::Fetch),
            ("builtin.put", Builtin::Replace), ("ext.builtin.echo", Builtin::Tell), ("ext.builtin.define", Builtin::Define),
            ("ext.builtin.var_dump", Builtin::Dump), ("ext.builtin.array", Builtin::Pack),
            ("ext.builtin.print_r", Builtin::Layout), ("ext.builtin.unset", Builtin::Erase), ("ext.builtin.array.front", Builtin::Lead),
            ("ext.builtin.isset", Builtin::Held), ("ext.builtin.empty", Builtin::Hollow),
            ("ext.builtin.exit", Builtin::Leave),
            ("ext.builtin.input", Builtin::Ask), ("ext.builtin.slice", Builtin::MakeSlice), ("ext.builtin.stream.write", Builtin::StreamPut), ("ext.builtin.stream.read", Builtin::StreamTake),
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
            ("ext.builtin.member.get", Builtin::GetAttr),
            ("ext.builtin.member.set", Builtin::SetAttr),
            ("ext.builtin.instance", Builtin::InstanceOf),
            ("ext.builtin.module.load", Builtin::ModuleLoad),
            ("ext.builtin.copy", Builtin::CopyValue),
            ("ext.builtin.class.derive", Builtin::DeriveClass),
            ("ext.builtin.call.outcome", Builtin::CallOutcome),
            ("ext.builtin.clock", Builtin::Clock),
            ("ext.builtin.room.used", Builtin::RoomUsed), ("ext.builtin.room.most", Builtin::RoomMost),
            ("ext.builtin.room.most.forget", Builtin::RoomForget), ("ext.builtin.room.limit", Builtin::RoomLimit),
            ("ext.builtin.file.read", Builtin::FileRead), ("ext.builtin.file.write", Builtin::FileWrite),
            ("ext.builtin.file.exists", Builtin::FileThere), ("ext.builtin.file.kind", Builtin::FileKind), ("ext.system.fault.current", Builtin::FaultInHand), ("ext.system.fault.held", Builtin::FaultItself), ("ext.builtin.host.info", Builtin::HostFacts), ("ext.builtin.file.remove", Builtin::FileGone),
            ("ext.builtin.shell", Builtin::ShellSaid),
            ("ext.builtin.net.ask", Builtin::NetAsk), ("ext.builtin.wait", Builtin::Waited),
            ("ext.builtin.run.begin", Builtin::RunBegin), ("ext.builtin.run.end", Builtin::RunEnd),
        ] {
            for lex in r.strings(tag)? {
                // A method spelled with its class before it (float.fromhex)
                // is a builtin word of its own; a bare method word is not.
                if native == Builtin::ValueMethod && !lex.contains('.') { continue; }
                let begins = lex.chars().next().map_or(false, |c| c == '_' || c.is_alphabetic());
                if !begins || lex.chars().any(|c| c.is_whitespace() || quotes.contains(&c)) {
                    return Err(format!("builtin name '{lex}' must begin like an identifier and hold no spaces or quotes"));
                }
                if natives.get(&lex) == Some(&native) { continue; }
                if let Some(prior) = natives.get(&lex).copied() {
                    if matches!(native, Builtin::Bytes(_) | Builtin::Text(_) | Builtin::ClassTool(_)) { continue; }
                    if matches!(prior, Builtin::Bytes(_) | Builtin::Text(_)) { natives.insert(lex.clone(), native); continue; }
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

        let core_words = ["core.integer", "core.not_iterator", "core.power.integer", "core.dict.sequence", "core.unindexable", "core.immutable", "core.power.zero", "core.power.overflow", "core.arity.one", "core.arity.exact", "key", "reverse", "start", "default", "round.ndigits", "round.number", "pow.base", "pow.exp", "pow.mod", "core.uniterable", "core.uncallable", "core.unhashable", "core.unready", "core.exhausted", "core.isinstance.amiss", "core.empty", "core.arity", "core.attribute", "core.attribute.name", "core.vars", "core.zero", "core.mod.zero", "core.inverse", "core.default.many", "core.dict.pair", "core.unsized", "core.dict.changed", "zip.strict", "zip.short", "zip.long"].into_iter().map(|n| Ok((n.to_string(), r.strings(&format!("ext.builtin.{}", n))?))).collect::<Result<HashMap<_, _>, String>>()?;
        let mut lang = Lang {
            core_words,
            ident: name,
            extensions: r.strings("extensions")?,
            banner: prefix,
            line_continuations: r.strings("ext.lexical.line_continuation")?,
            bare_number_point: r.flag("ext.lexical.number.point.bare")?,
            separator_after_prefix: r.flag("ext.lexical.number.separator.after_prefix")?,
            whole_bits: r.flag("ext.op.bit.whole")?,
            print_real_point: r.flag("ext.builtin.print.real_point")?,
            with_as: r.strings("ext.stmt.with.as")?,
            with_unready: r.strings("ext.stmt.with.unready")?,
            yield_from: r.strings("ext.stmt.yield.from")?,
            member_pipes: r.flag("ext.op.member.pipes")?,
            tuple_unready: r.strings("ext.op.tuple.unready")?,
            byte_words: ["ext.builtin.bytes.signed", "ext.system.bytes.strict", "ext.builtin.bytes", "ext.builtin.bytearray", "ext.builtin.bytes.encode", "ext.builtin.bytes.decode", "ext.builtin.bytes.hex", "ext.builtin.bytes.fromhex", "ext.builtin.bytes.upper", "ext.builtin.bytes.lower", "ext.builtin.bytes.split", "ext.builtin.bytes.join", "ext.builtin.bytes.startswith", "ext.builtin.bytes.replace", "ext.builtin.bytes.strip", "ext.builtin.bytes.find", "ext.builtin.bytes.from_int", "ext.builtin.bytes.to_int", "ext.builtin.isinstance", "ext.builtin.hash", "ext.system.bytes.repr", "ext.system.bytes.type", "ext.system.bytes.encodings", "ext.system.bytes.order", "ext.system.bytes.unready", "ext.system.bytes.arguments", "ext.system.bytes.range", "ext.system.bytes.negative", "ext.system.bytes.index", "ext.system.bytes.immutable", "ext.system.bytes.unhashable", "ext.system.bytes.separator", "ext.system.bytes.hex", "ext.system.bytes.overflow", "ext.system.bytes.unsigned", "ext.system.bytes.bad_order", "ext.system.bytes.decode", "ext.system.bytes.encode", "ext.lexical.string.bytes.ascii", "ext.lexical.string.bytes.mixed"].iter().map(|key| Ok((key.to_string(), r.strings(key)?))).collect::<Result<_, String>>()?,
            bytes_unready: r.strings("ext.lexical.string.prefix.bytes.unready")?,
            fmt_op_rem_format_nan: r.strings("ext.op.rem.format.nan")?,
            fmt_op_rem_format_infinity: r.strings("ext.op.rem.format.infinity")?,
            fmt_text_format_zero_integer: r.strings("ext.text.format.zero.integer")?,
            fmt_text_format_zero_string: r.strings("ext.text.format.zero.string")?,
            format_builtin: r.strings("ext.builtin.format")?,
            format_method: r.strings("ext.text.format")?,
            fmt_text_format_invalid: r.strings("ext.text.format.invalid")?,
            fmt_text_format_unknown: r.strings("ext.text.format.unknown")?,
            fmt_text_format_kinds: r.strings("ext.text.format.kinds")?,
            fmt_text_format_unready: r.strings("ext.text.format.unready")?,
            fmt_text_format_precision_integer: r.strings("ext.text.format.precision.integer")?,
            fmt_text_format_precision_missing: r.strings("ext.text.format.precision.missing")?,
            fmt_text_format_sign_string: r.strings("ext.text.format.sign.string")?,
            fmt_text_format_alternate_string: r.strings("ext.text.format.alternate.string")?,
            fmt_text_format_align_string: r.strings("ext.text.format.align.string")?,
            fmt_text_format_sign_character: r.strings("ext.text.format.sign.character")?,
            fmt_text_format_alternate_character: r.strings("ext.text.format.alternate.character")?,
            fmt_text_format_character: r.strings("ext.text.format.character")?,
            fmt_text_format_spec_type: r.strings("ext.text.format.spec.type")?,
            fmt_text_format_numbered_auto: r.strings("ext.text.format.numbered.auto")?,
            fmt_text_format_numbered_manual: r.strings("ext.text.format.numbered.manual")?,
            fmt_text_format_index: r.strings("ext.text.format.index")?,
            fmt_text_format_key: r.strings("ext.text.format.key")?,
            fmt_text_format_brace_open: r.strings("ext.text.format.brace.open")?,
            fmt_text_format_brace_close: r.strings("ext.text.format.brace.close")?,
            fmt_text_format_conversion: r.strings("ext.text.format.conversion")?,
            fmt_text_format_recursion: r.strings("ext.text.format.recursion")?,
            fmt_op_rem_format_few: r.strings("ext.op.rem.format.few")?,
            fmt_op_rem_format_many: r.strings("ext.op.rem.format.many")?,
            fmt_op_rem_format_mapping: r.strings("ext.op.rem.format.mapping")?,
            fmt_op_rem_format_number: r.strings("ext.op.rem.format.number")?,
            fmt_op_rem_format_integer: r.strings("ext.op.rem.format.integer")?,
            fmt_op_rem_format_real: r.strings("ext.op.rem.format.real")?,
            fmt_op_rem_format_character: r.strings("ext.op.rem.format.character")?,
            fmt_op_rem_format_star: r.strings("ext.op.rem.format.star")?,
            fmt_op_rem_format_incomplete: r.strings("ext.op.rem.format.incomplete")?,
            fmt_op_rem_format_code: r.strings("ext.op.rem.format.code")?,
            format_unready: r.strings("ext.lexical.string.prefix.format.unready")?,
            identity_unready: r.strings("ext.op.identical.unsupported")?,
            in_values: r.strings("ext.op.in")?,
            in_not: r.strings("ext.op.in.negated")?,
            in_unready: r.strings("ext.op.in.unsupported")?,
            if_else: r.strings("ext.op.if_else")?,
            assign_chain: r.flag("ext.stmt.assign.chain")?,
            deferred_escapes: r.letters("ext.lexical.escape.deferred")?,
            imaginary_suffixes: r.strings("ext.lexical.number.imaginary")?,
            await_words: r.strings("ext.op.await")?,
            async_words: r.strings("ext.stmt.async")?,
            match_words: r.strings("ext.stmt.match")?,
            match_cases: r.strings("ext.stmt.match.case")?,
            type_alias_words: r.strings("ext.stmt.type_alias")?,
            type_parameters: r.flag("ext.stmt.type_parameters")?,
            pipe_attribute: r.flag("ext.op.pipe.attribute")?,
            line_comments: r.strings("lexical.comment_line")?,
            block_comments: comment_opens.into_iter().zip(comment_closes).collect(),
            quotes,
            long_quotes: r.strings("ext.lexical.string.long")?,
            adjacent_strings: r.flag("ext.lexical.string.adjacent")?,
            nonlocal_words: r.strings("ext.stmt.nonlocal")?,
            nonlocal_unrun: r.strings("ext.stmt.nonlocal.unrun")?,
            with_words: r.strings("ext.stmt.with")?,
            with_as_words: r.strings("ext.stmt.with.as")?,
            with_unrun: r.strings("ext.stmt.with.unrun")?,
            async_unrun: r.strings("ext.stmt.async.unrun")?,
            del_words: r.strings("ext.stmt.del")?,
            del_unrun: r.head("ext.stmt.del.unrun")?.unwrap_or_default(),
            continued_escapes: r.flag("ext.lexical.escape.continued")?,
            loop_else: r.flag("ext.stmt.loop.else")?,
            yield_words: r.strings("ext.stmt.yield")?,
            yield_from_words: r.strings("ext.stmt.yield.from")?,
            raw_quotes,
            raw_prefixes: r.letters("ext.lexical.string.prefix.raw")?,
            byte_prefixes: r.letters("ext.lexical.string.prefix.bytes")?,
            plain_prefixes: r.letters("ext.lexical.string.prefix.plain")?,
            format_prefixes: r.letters("ext.lexical.string.prefix.format")?,
            string_amiss: r.head("ext.lexical.string.amiss")?,
            bytes_unavailable: r.head("ext.lexical.string.bytes.unavailable")?,
            format_unavailable: r.head("ext.lexical.string.format.unavailable")?,
            string_unready: r.head("ext.lexical.string.value.unready")?,
            range_zero_start: r.flag("ext.builtin.range.zero_start")?,
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
            imaginary_letters: r.letters("ext.lexical.number.imaginary")?,
            complex_words: ["ext.builtin.complex", "ext.builtin.complex.real", "ext.builtin.complex.imag", "ext.builtin.method.conjugate", "ext.builtin.complex.invalid", "ext.builtin.complex.integer", "ext.builtin.complex.order", "ext.builtin.complex.floor", "ext.builtin.complex.zero", "ext.builtin.complex.power.zero", "ext.builtin.complex.unready"].into_iter().map(|label| Ok((label.to_string(), r.strings(label)?))).collect::<Result<_, String>>()?,
            imaginary_unready: r.head("ext.lexical.number.imaginary.unready")?,
            number_point_edge: r.flag("ext.lexical.number.point_edge")?,
            number_strict: r.flag("ext.lexical.number.separator.strict")?,
            number_leading_zero: r.head("ext.lexical.number.amiss.leading_zero")?,
            binary_amiss: r.head("ext.lexical.number.amiss.binary")?,
            binary_digit_amiss: r.strings("ext.lexical.number.amiss.binary.digit")?,
            octal_amiss: r.head("ext.lexical.number.amiss.octal")?,
            octal_digit_amiss: r.strings("ext.lexical.number.amiss.octal.digit")?,
            hex_amiss: r.head("ext.lexical.number.amiss.hex")?,

            prologue: r.head("lexical.prologue")?,
            open_decimal_point: r.flag("ext.lexical.number.point.open")?,
            point: r.letter("lexical.number.decimal_point")?,
            bare_point: r.flag("ext.lexical.number.point.bare")?,
            base_mark: r.letter("lexical.number.base_marker")?,
            exponent_mark: r.letter("lexical.number.exponent_marker")?,
            hex_prefix,
            base_prefixes,
            octal_lead: r.flag("ext.lexical.number.octal_lead")?,
            digit_separators: r.letters("ext.lexical.number.separator")?,
            integer_bits: r.count("ext.system.integer.bits")?,
            arithmetic_binary: r.flag("ext.op.arithmetic.binary")?,
            arithmetic_flags: r.flag("ext.op.arithmetic.flags")?,
            infinity_words: r.strings("ext.builtin.to_real.infinity")?,
            nan_words: r.strings("ext.builtin.to_real.nan")?,
            point_open: r.flag("ext.lexical.number.point_open")?,
            power_real: r.flag("ext.op.pow.real_exponent")?,
            power_overflow: r.strings("ext.op.pow.overflow")?,
            power_nonreal: r.strings("ext.op.pow.nonreal")?,
            power_zero: r.strings("ext.op.pow.zero")?,
            division_zero: r.strings("ext.op.div.zero")?,
            quotient_zero: r.strings("ext.op.quot.zero")?,
            quotient_real_zero: r.strings("ext.op.quot.real_zero")?,
            remainder_real_zero: r.strings("ext.op.rem.real_zero")?,
            integer_text_detail: r.strings("ext.builtin.to_int.text.detail")?,
            bits_unbounded: r.flag("ext.op.bit.unbounded")?,
            bits_integer: r.strings("ext.op.bit.integer")?,
            bits_beyond: r.strings("ext.op.bit.beyond")?,
            real_bits: r.count("ext.system.real.bits")?,
            shortest_reals: match r.string("system.real.render")?.as_str() {
                "shortest" => true, "library" => false,
                _ => return Err("system.real.render must be 'library' or 'shortest'".into()),
            },
            real_digits: r.count("ext.system.real.digits")?,
            figures_binding: r.head("ext.system.real.figures")?,
            figures_shown_binding: r.head("ext.system.real.figures.shown")?,
            unicode_names: unicode,
            sigil: var_prefix,
            keywords_folded: r.flag("lexical.keywords_case_insensitive")?,
            names_folded: r.flag("identifier.case_insensitive")?,
            quote_for_names: r.letter("lexical.name_quote")?,
            symbols: Vec::new(),
            continuation_amiss: r.strings("ext.lexical.line_continuation.amiss")?,
            type_params_open: r.strings("ext.stmt.type_params.open")?,
            type_params_close: r.strings("ext.stmt.type_params.close")?,
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
            whole_bits_amiss: r.head("ext.op.bit.whole.amiss")?,
            whole_bits_large: r.head("ext.op.bit.whole.large")?,
            matrix_words: r.strings("ext.op.matrix")?,
            matrix_unavailable: r.head("ext.op.matrix.unavailable")?,
            index_spread_unsupported: r.head("ext.op.index.spread.unsupported")?,
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
            method_keywords: [("ext.builtin.method.sort.key", "key"), ("ext.builtin.method.sort.reverse", "reverse"), ("ext.builtin.method.split.sep", "sep"), ("ext.builtin.method.split.maxsplit", "maxsplit")].into_iter().map(|(label, purpose)| Ok((purpose, r.strings(label)?))).collect::<Result<Vec<_>, String>>()?.into_iter().flat_map(|(purpose, words)| words.into_iter().map(move |word| (word, purpose.to_string()))).collect(),
            value_methods: ["indices", "slice_hash", "bit_count", "numerator", "denominator", "real", "imag", "__index__", "__truediv__", "fromhex", "conjugate", "upper", "lower", "strip", "lstrip", "rstrip", "split", "rsplit", "join", "replace", "startswith", "endswith", "find", "rfind", "index", "count", "isdigit", "isalpha", "isalnum", "isspace", "islower", "isupper", "title", "capitalize", "center", "ljust", "rjust", "zfill", "format", "encode", "append", "extend", "insert", "pop", "remove", "sort", "reverse", "copy", "clear", "get", "keys", "values", "items", "setdefault", "update", "popitem", "fromkeys", "bit_length", "is_integer", "hex", "as_integer_ratio"].into_iter().map(|n| Ok((n, r.strings(&format!("ext.builtin.method.{n}"))?))).collect::<Result<Vec<_>, String>>()?.into_iter().flat_map(|(n, words)| words.into_iter().map(move |word| (word, n.to_string()))).collect(),
            method_errors: ["unready", "bytes", "arguments", "separator", "substring", "pop", "index", "remove", "list_index", "format", "spec", "unicode", "attribute", "key", "missing", "mixed", "fill", "hex", "hex_overflow", "popitem"].into_iter().map(|n| Ok((n.to_string(), r.head(&format!("ext.builtin.method.error.{n}"))?.unwrap_or_default()))).collect::<Result<_, String>>()?,
            pipe_words: pipes,
            range_marks: ranges,
            precedence: syntax_tiers,
            assign_words: r.strings("stmt.assign")?,
            let_words: lets,
            mutable_words: mutables,
            type_marks: annotation,
            annotation_marks: r.strings("ext.stmt.annotation")?,
            annotation_amiss: r.head("ext.stmt.annotation.amiss")?,
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
            async_functions: r.strings("ext.stmt.function.async")?,
            async_functions_unavailable: r.strings("ext.stmt.function.async.unavailable")?,
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
            text_negative_index: r.flag("ext.op.index.text.negative")?,
            text_repeat: r.flag("ext.builtin.text.repeat")?,
            text_words: ["ext.builtin.text.fault.protocol","ext.builtin.text.fault.index","ext.builtin.text.splitlines","ext.builtin.text.partition","ext.builtin.text.rpartition","ext.builtin.text.expandtabs","ext.builtin.text.swapcase","ext.builtin.text.casefold","ext.builtin.text.capitalize","ext.builtin.text.title","ext.builtin.text.istitle","ext.builtin.text.isidentifier","ext.builtin.text.isprintable","ext.builtin.text.isdecimal","ext.builtin.text.isnumeric","ext.builtin.text.isascii","ext.builtin.text.removeprefix","ext.builtin.text.removesuffix","ext.builtin.text.format_map","ext.builtin.text.maketrans","ext.builtin.text.translate","ext.builtin.text.encode","ext.builtin.text.join","ext.builtin.text.split","ext.builtin.text.rsplit","ext.builtin.text.strip","ext.builtin.text.lstrip","ext.builtin.text.rstrip","ext.builtin.text.center","ext.builtin.text.ljust","ext.builtin.text.rjust","ext.builtin.text.zfill","ext.builtin.text.count","ext.builtin.text.find","ext.builtin.text.rfind","ext.builtin.text.index","ext.builtin.text.rindex","ext.builtin.text.startswith","ext.builtin.text.endswith","ext.builtin.text.replace","ext.builtin.text.upper","ext.builtin.text.lower","ext.builtin.text.length","ext.builtin.text.repr","ext.builtin.text.keyword.keepends","ext.builtin.text.keyword.tabsize","ext.builtin.text.keyword.maxsplit","ext.builtin.text.keyword.sep","ext.builtin.text.keyword.encoding","ext.builtin.text.keyword.errors","ext.builtin.text.fault.arguments","ext.builtin.text.fault.receiver","ext.builtin.text.fault.string","ext.builtin.text.fault.integer","ext.builtin.text.fault.separator","ext.builtin.text.fault.fill","ext.builtin.text.fault.missing","ext.builtin.text.fault.encode","ext.builtin.text.fault.mapping","ext.builtin.text.fault.translation","ext.builtin.text.fault.codepoint","ext.builtin.text.fault.surrogate","ext.builtin.text.fault.maketrans.length","ext.builtin.text.fault.maketrans.key","ext.builtin.text.fault.maketrans.type","ext.builtin.text.fault.format","ext.builtin.text.fault.format.positional","ext.builtin.text.fault.format.brace","ext.builtin.text.fault.join","ext.builtin.text.fault.walk","ext.builtin.text.fault.room","ext.builtin.text.fault.key","ext.builtin.text.complaint"].into_iter().map(|k| Ok((k.to_string(), r.strings(k)?))).collect::<Result<_, String>>()?,
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
            exceptions: r.strings("ext.builtin.exceptions")?,
            exception_args: r.head("ext.builtin.exceptions.args")?,
            exception_cause: r.head("ext.builtin.exceptions.cause")?,
            exception_context: r.head("ext.builtin.exceptions.context")?,
            exception_suppress: r.head("ext.builtin.exceptions.suppress")?,
            exception_traceback: r.head("ext.builtin.exceptions.traceback")?,
            note_method: r.head("ext.builtin.exceptions.note")?,
            notes_member: r.head("ext.builtin.exceptions.notes")?,
            note_invalid: r.head("ext.builtin.exceptions.note.invalid")?,
            traceback_member: r.head("ext.builtin.exceptions.traceback.member")?,
            traceback_setter: r.head("ext.builtin.exceptions.traceback.with")?,
            absent_name_member: r.head("ext.builtin.exceptions.name")?,
            absent_object_member: r.head("ext.builtin.exceptions.object")?,
            os_members: r.strings("ext.builtin.exceptions.os")?,
            os_message: r.strings("ext.builtin.exceptions.os.message")?,
            group_message: r.head("ext.builtin.exceptions.group.message")?,
            group_members: r.head("ext.builtin.exceptions.group.members")?,
            group_split: r.head("ext.builtin.exceptions.group.split")?,
            group_subgroup: r.head("ext.builtin.exceptions.group.subgroup")?,
            group_derive: r.head("ext.builtin.exceptions.group.derive")?,
            group_summary: r.strings("ext.builtin.exceptions.group.summary")?,
            group_invalid: r.head("ext.builtin.exceptions.group.invalid")?,
            class_name: r.head("ext.builtin.class.name")?,
            exception_unready: r.head("ext.builtin.exceptions.unready")?,
            division_words: r.head("ext.system.fault.division")?,
            index_words: r.head("ext.system.fault.index")?,
            name_words: r.strings("ext.system.fault.name")?,
            attribute_words: r.strings("ext.system.fault.attribute")?,
            kind_words: r.head("ext.system.fault.kind")?,
            fault_index: r.head("ext.system.fault.class.index")?,
            fault_key: r.head("ext.system.fault.class.key")?,
            marks_lines: r.flag("ext.system.source.marked")?,
            fault_name: r.head("ext.system.fault.class.name")?,
            fault_attribute: r.head("ext.system.fault.class.attribute")?,
            fault_stop: r.head("ext.system.fault.class.stop")?,
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
            bit_room: r.head("ext.op.bit.whole.room")?,
            plus_non_number: r.head("ext.op.plus.non_number")?,
            left_shift_unready: r.head("ext.op.bit.left.unready")?,
            bit_operands: r.head("ext.op.bit.operands")?,
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
            ellipsis_words: r.strings("ext.literal.ellipsis")?,
            ellipsis_unready: r.head("ext.literal.ellipsis.unready")?,
            unimplemented_words: r.strings("ext.literal.unimplemented")?,
            shadow_builtins: r.flag("ext.syntax.names.shadow_builtins")?,
            integer_digits: r.count("ext.builtin.to_int.digits")?,
            digits_amiss: r.strings("ext.builtin.to_int.digits.amiss")?,
            to_int_infinity: r.head("ext.builtin.to_int.infinity")?,
            to_int_nan: r.head("ext.builtin.to_int.nan")?,
            round_whole_even: r.flag("ext.builtin.round.whole.even")?,
            bool_base: r.head("ext.builtin.bool.base")?,
            builtin_bases: r.strings("ext.stmt.class.builtin")?,
            layout_amiss: r.head("ext.stmt.class.layout")?,
            missing_key: r.head("ext.stmt.class.missing")?,
            bool_result: r.head("ext.builtin.bool.result")?,
            order_unsupported: r.strings("ext.op.order.unsupported")?,
            lambda_words: r.strings("ext.op.lambda")?,
            lambda_unready: r.head("ext.op.lambda.unready")?,
            expression_assign: r.strings("ext.op.assign.expression")?,
            chained_calls: r.flag("ext.syntax.call.chained")?,
            chained_names: r.flag("ext.stmt.assign.names.chained")?,
            tuple_marks: r.strings("ext.op.tuple")?,
            matrix_unready: r.head("ext.op.matrix.unready")?,
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
            class_bases_open: r.strings("ext.stmt.class.bases.open")?,
            class_bases_close: r.strings("ext.stmt.class.bases.close")?,
            special_stop: r.strings("ext.stmt.class.special.stop")?,
            special_declined: r.strings("ext.stmt.class.special.declined")?,
            special_unready: r.strings("ext.stmt.class.special.unready")?,
            class_special: r.strings("ext.stmt.class.special")?,
            special_amiss: r.strings("ext.stmt.class.special.amiss")?,
            index_answer_amiss: r.strings("ext.stmt.class.index.amiss")?,
            operands_amiss: r.strings("ext.stmt.class.binary.amiss")?,
            format_spec_amiss: r.strings("ext.stmt.class.format.amiss")?,
            metaclass_word: r.strings("ext.stmt.class.metaclass")?,
            class_unready: r.strings("ext.stmt.class.unready")?,
            yield_suspends: r.flag("ext.stmt.yield.suspends")?,
            yield_exhausted: r.strings("ext.stmt.yield.exhausted")?,
            yield_send: r.strings("ext.stmt.yield.send")?,
            yield_close: r.strings("ext.stmt.yield.close")?,
            yield_throw: r.strings("ext.stmt.yield.throw")?,
            yield_unstarted: r.strings("ext.stmt.yield.unstarted")?,
            yield_busy: r.strings("ext.stmt.yield.busy")?,
            yield_unsupported: r.strings("ext.stmt.yield.unsupported")?,
            yield_throw_unavailable: r.strings("ext.stmt.yield.throw.unavailable")?,
            with_enter: r.head("ext.stmt.with.enter")?,
            with_leave: r.head("ext.stmt.with.leave")?,
            yield_unrun: r.strings("ext.stmt.yield.unrun")?,
            scope_unready: r.strings("ext.system.scope.unready")?,
            global_words: r.strings("ext.stmt.global")?,
            binding_unrun: r.head("ext.stmt.binding.unrun")?.unwrap_or_default(),
            import_values: r.flag("ext.stmt.import.value")?,
            import_missing: r.strings("ext.stmt.import.missing")?,
            import_member_missing: r.strings("ext.stmt.import.member.missing")?,
            import_relative_unready: r.head("ext.stmt.import.relative.unready")?.unwrap_or_default(),
            import_words: r.strings("ext.stmt.import")?,
            import_from_words: r.strings("ext.stmt.import.from")?,
            import_as_words: r.strings("ext.stmt.import.as")?,
            math_floating: r.flag("ext.builtin.math.floating")?,
            module_helper_amiss: r.head("ext.builtin.module.helper.amiss")?.unwrap_or_default(),
            member_absent: r.strings("ext.builtin.member.absent")?,
            module_cache: r.strings("ext.system.module.cache")?,
            module_names: r.strings("ext.system.module.name")?,
            module_doc: r.strings("ext.system.module.doc")?,
            module_builtins: r.strings("ext.system.module.builtins")?,
            compile_modes: r.strings("ext.builtin.compile.modes")?,
            compile_parameters: r.strings("ext.builtin.compile.parameters")?,
            compile_kind: r.head("ext.builtin.compile.kind")?,
            source_syntax: r.head("ext.builtin.source.syntax")?,
            source_unready: r.head("ext.builtin.source.unready")?,
            class_details: ["call", "locals", "root", "mro", "order", "name", "qualified", "bases", "namespace", "kind", "allocate", "subclass", "slots", "set", "remove", "get", "getitem", "doc", "module", "defaults", "code", "argcount", "varnames", "receiver", "function", "main", "mro.amiss", "attribute.amiss", "unready", "descriptor.get", "descriptor.set", "descriptor.delete", "descriptor.name", "descriptor.foreign", "property.fget", "property.fset", "property.fdel", "property.getter", "property.deleter", "property.doc", "property.readonly", "property.unreadable", "property.unwritable", "property.undeletable"].into_iter().map(|part| Ok((part.to_string(), r.strings(&format!("ext.stmt.class.detail.{}", part))?))).collect::<Result<_, String>>()?,
            decorator_words: r.strings("ext.stmt.decorator")?,
            decorator_amiss: r.head("ext.stmt.decorator.amiss")?,
            const_words: r.strings("ext.stmt.const")?,
            match_wildcards: r.strings("ext.stmt.match.wildcard")?,
            match_ors: r.strings("ext.stmt.match.or")?,
            match_guards: r.strings("ext.stmt.match.guard")?,
            match_as: r.strings("ext.stmt.match.as")?,
            match_unready: r.strings("ext.stmt.match.unready")?,
            match_invalid: r.strings("ext.stmt.match.invalid")?,
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
            closes_over: r.flag("ext.stmt.function.closes_over")?,
            local_unbound: r.strings("ext.stmt.function.local.unbound")?,
            free_unbound: r.strings("ext.stmt.function.free.unbound")?,
            nonlocal_amiss: r.strings("ext.stmt.nonlocal.amiss")?,
            nonlocal_module: r.head("ext.stmt.nonlocal.module")?,
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
            imaginary_unrun: r.head("ext.lexical.number.imaginary.unrun")?.unwrap_or_default(),
            plus_words: r.strings("ext.op.plus")?,
            if_else_words: r.strings("ext.op.if_else")?,
            lambda_enclosing: r.head("ext.op.lambda.enclosing")?,
            identity_not: r.strings("ext.op.identical.negated")?,
            identity_unsupported: r.head("ext.op.identical.unsupported")?,
            membership_words: r.strings("ext.op.in")?,
            membership_not: r.strings("ext.op.in.negated")?,
            membership_unsupported: r.head("ext.op.in.unsupported")?,
            chained_comparisons: r.flag("ext.op.compare.chained")?,
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
            tuple_separator: r.head("ext.op.tuple")?,
            or_maps: r.flag("ext.op.bit.or.maps")?,
            unordered_maps: r.flag("ext.op.eq.maps.unordered")?,
            value_keys: r.flag("ext.syntax.map.value_keys")?,
            map_unhashable: r.strings("ext.syntax.map.unhashable")?,
            map_resized: r.head("ext.syntax.map.resized")?,
            print_separator: r.head("ext.builtin.print.separator")?,
            print_ending: r.head("ext.builtin.print.end")?,
            print_option_type: r.head("ext.builtin.print.option.type")?,
            map_argument_amiss: r.head("ext.builtin.map.arguments.amiss")?,
            map_pair_amiss: r.head("ext.builtin.map.pair.amiss")?,
            set_words: ["ext.builtin.set.method.unavailable","ext.builtin.set.changed","ext.builtin.set","ext.builtin.set.add","ext.builtin.set.remove","ext.builtin.set.discard","ext.builtin.set.pop","ext.builtin.set.clear","ext.builtin.set.copy","ext.builtin.set.update","ext.builtin.set.union","ext.builtin.set.intersection","ext.builtin.set.difference","ext.builtin.set.symmetric_difference","ext.builtin.set.issubset","ext.builtin.set.issuperset","ext.builtin.set.isdisjoint","ext.builtin.set.intersection_update","ext.builtin.set.difference_update","ext.builtin.set.symmetric_difference_update","ext.builtin.set.sorted","ext.builtin.set.unhashable","ext.builtin.set.missing","ext.builtin.set.empty","ext.builtin.set.operands","ext.builtin.set.arguments","ext.builtin.set.unsupported","ext.builtin.set.unsortable"].into_iter().map(|label| Ok((label.to_string(), r.strings(label)?))).collect::<Result<_, String>>()?,
            array_spread: r.strings("ext.syntax.array.spread")?,
            map_spread: r.strings("ext.syntax.map.spread")?,
            collection_unwalkable: r.strings("ext.syntax.collection.unwalkable")?,
            spread_unmapped: r.strings("ext.syntax.map.spread.unmapped")?,
            comprehension_unpack_amiss: r.strings("ext.op.comprehension.unpack.amiss")?,

            class_words: r.strings("ext.stmt.class")?,
            bases_open: r.head("ext.stmt.class.bases.open")?,
            bases_close: r.head("ext.stmt.class.bases.close")?,
            explicit_this: r.flag("ext.stmt.class.this.explicit")?,
            class_static: r.strings("ext.stmt.class.static")?,
            class_method: r.strings("ext.stmt.class.classmethod")?,
            class_property: r.strings("ext.stmt.class.property")?,
            property_setter: r.strings("ext.stmt.class.property.setter")?,
            extends_words: r.strings("ext.stmt.class.extends")?,
            new_words: r.strings("ext.stmt.class.new")?,
            this_word: r.head("ext.stmt.class.this")?,
            constructor: r.head("ext.stmt.class.constructor")?,
            destructor: r.head("ext.stmt.class.destructor")?,
            reader: r.head("ext.stmt.class.reader")?,
            module_getattr: r.head("ext.system.module.getattr")?,
            class_annotations: r.strings("ext.stmt.class.annotations")?,
            class_called: r.head("ext.stmt.class.called")?,
            text_ordered: r.flag("ext.op.order.text")?,
            slice_parts: ["ext.builtin.slice", "ext.builtin.slice.start", "ext.builtin.slice.stop", "ext.builtin.slice.step", "ext.builtin.slice.arity", "ext.builtin.slice.length", "ext.op.index.integer", "ext.op.index.slice.amiss"].into_iter().map(|k| Ok((k.to_string(), r.head(k)?.unwrap_or_default()))).collect::<Result<_, String>>()?,
            class_walked: r.head("ext.stmt.class.walked")?,
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
            print_route: r.strings("ext.builtin.print.redirect")?,
            input_route: r.strings("ext.builtin.input.reader")?,
            stream_amiss: r.strings("ext.builtin.stream.amiss")?,
            stream_failed: r.strings("ext.builtin.stream.failed")?,
            clock_parts: r.flag("ext.builtin.clock.parts")?,
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
            throw_invalid: r.head("ext.stmt.throw.invalid")?,
            catch_amiss: r.head("ext.stmt.catch.amiss")?,
            yield_escaped: r.head("ext.stmt.yield.escaped")?,
            with_invalid: r.strings("ext.stmt.with.invalid")?,
            recursion_limit: r.count("ext.system.recursion.limit")?,
            recursion_exceeded: r.head("ext.system.recursion.exceeded")?,
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
        if lang.type_params_open.is_empty() != lang.type_params_close.is_empty() {
            return Err("ext.stmt.type_params needs both opening and closing brackets".into());
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
                if matches!(op.action, Action::Contains) { continue; }
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
        for lex in &self.matrix_words {
            place(lex);
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
        for mark in [&self.tuple_separator, &self.bases_open, &self.bases_close, &self.member_mark, &self.scope_mark, &self.catch_between, &self.catch_tuple_open, &self.catch_tuple_close, &self.catch_group, &self.reference_mark, &self.otherwise_mark].into_iter().flatten() {
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
            &self.type_params_open, &self.type_params_close, &self.comprehension_async, &self.comprehension_for, &self.comprehension_in, &self.comprehension_if, &self.array_spread, &self.map_spread, &self.block_intros, &self.assign_words, &self.stmt_ends, &self.argument_labels, &self.type_marks, &self.annotation_marks, &self.return_marks, &self.if_else_words, &self.lambda_words, &self.identity_not, &self.membership_words, &self.membership_not, &self.expression_assign, &self.ellipsis_words, &self.dup_words, &self.drop_words, &self.swap_words, &self.over_words, &self.rot_words, &self.eval_words, &self.quote_open, &self.long_quotes, &self.tuple_marks, &self.class_bases_open, &self.class_bases_close, &self.del_words, &self.nonlocal_words, &self.with_words, &self.with_as_words, &self.yield_words, &self.yield_from_words, &self.slice_ellipsis, &self.slice_marks, &self.quote_close, &self.increments, &self.decrements, &self.case_marks, &self.decorator_words, &self.carries_words, &self.carries_pairs, &self.keyword_only, &self.positional_only, &self.call_spread, &self.call_spread_pairs, &self.stmt_separators, &self.unpack_rest, &self.unpack_words, &self.line_continuations, &self.match_ors, &self.matrix_words, &self.async_functions, &self.await_words, &self.async_words, &self.match_words, &self.match_cases, &self.type_alias_words,
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
