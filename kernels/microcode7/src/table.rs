// The definition as data. Every label is read into a typed cell, checked
// for shape, and a few tables are derived for the stages.

use std::collections::{HashMap, HashSet};

use serde_json::Value as Json;

use crate::form::Prim;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blocks {
    Indented,
    Bracketed,
    Worded,
}

#[derive(Debug)]
enum Entry {
    Strings(Vec<String>),
    Flag(bool),
    Count(Option<usize>),
    One(Option<String>),
    Tiers(Vec<Vec<String>>),
}

#[derive(Clone, Copy, Debug)]
pub struct Infix {
    pub prim: Prim,
    pub level: u32,
    pub right_assoc: bool,
}

pub struct Table {
    pub ident: String,
    pub blocks: Blocks,
    /// Reverse Polish: words over one stack, no expressions.
    pub rpn: bool,
    cells: HashMap<&'static str, Entry>,
    pub dyadic: HashMap<String, Infix>,
    pub monadic: HashMap<String, Infix>,
    pub precedence: HashMap<String, u32>,
    pub prims: HashMap<String, Prim>,
    /// `x op= e` for each binary operator (ext.op.assign.compound).
    pub compound: HashMap<String, Prim>,
    /// The labels the definition wrote out, empty ones included.
    given: HashSet<String>,
    pub keywords: HashSet<String>,
    pub signs: Vec<String>,
}

// Label shapes: L list of words, B boolean, N count or null, W word, O word or null, T tiers.
const TAGS: &str = "\
format_version:N language:W extensions:L lexical.comment_line:L lexical.comment_block.open:L \
lexical.comment_block.close:L lexical.string_quotes:L lexical.raw_quotes:L lexical.string_escapes:L lexical.prologue:L \
lexical.name_quote:L lexical.number.decimal_point:L lexical.number.base_marker:L lexical.number.exponent_marker:L lexical.number.hex_prefix:L \
lexical.keywords_case_insensitive:B identifier.unicode:B identifier.variable_prefix:L identifier.case_insensitive:B block.style:W \
block.open:L block.close:L block.intro:L block.indent_size:N stmt.terminator:L \
syntax.notation:W syntax.group.open:L syntax.group.close:L syntax.call.open:L syntax.call.separator:L \
syntax.call.close:L syntax.call.label:L syntax.array.open:L syntax.array.separator:L syntax.array.close:L \
syntax.map.open:L syntax.map.separator:L syntax.map.pair:L syntax.map.close:L system.flag.counts:B literal.true:L \
literal.false:L literal.null:L literal.null.silent:B op.precedence:T op.right_associative:L op.add:L \
op.sub:L op.mul:L op.div:L op.div.result:O op.mod.whole:B op.quot:L \
op.rem:L op.pow:L op.eq:L op.ne:L op.lt:L \
op.le:L op.gt:L op.ge:L op.and:L op.or:L \
op.not:L op.negate:L op.concat:L op.range:L op.index.open:L \
op.index.close:L op.index.strings:B op.pipe:L stmt.assign:L stmt.let:L \
stmt.let.mutable:L stmt.let.annotation:L stmt.let.type_first:B stmt.if:L stmt.elif:L \
stmt.else:L stmt.while:L stmt.until:L stmt.for:L stmt.for.in:L \
stmt.foreach:L stmt.foreach.as:L stmt.foreach.pair:L stmt.return:L stmt.break:L \
stmt.continue:L stmt.function:L stmt.function.returns:L stmt.function.result_by_name:B stmt.pass:L \
stmt.emit:L stack.dup:L stack.drop:L stack.swap:L stack.over:L \
stack.rot:L stack.eval:L stack.program.open:L stack.program.close:L builtin.emit:L \
builtin.print:L builtin.write:L builtin.print.placeholder:L builtin.len:L builtin.char_at:L \
builtin.ord:L builtin.chr:L builtin.typeof:L builtin.error:L builtin.extern:L \
builtin.range:L builtin.real:L builtin.num:L builtin.den:L builtin.push:L \
builtin.get:L builtin.put:L builtin.precision:L builtin.to_string:L builtin.to_int:L \
builtin.to_real:L system.args:L system.memoization:L system.real_default_precision:L system.entry:L \
system.kind.integer:L system.kind.rational:L system.kind.real:L system.kind.string:L system.kind.boolean:L \
system.kind.array:L system.kind.null:L \
";

/// Extension labels beyond the core: optional, an absent one is empty
/// (or off). The reference kernels skip them; this one reads them.
const EXT_TAGS: &str = "\
ext.lexical.string.long:L ext.op.lambda:L ext.op.tuple:L ext.stmt.class.bases.open:L ext.stmt.class.bases.close:L ext.stmt.class.unready:L ext.stmt.del:L ext.stmt.nonlocal:L ext.stmt.nonlocal.unrun:L ext.stmt.with:L ext.stmt.with.as:L ext.stmt.yield:L ext.stmt.yield.from:L ext.stmt.yield.unrun:L ext.system.scope.unready:L ext.op.index.slice.ellipsis:L ext.op.index.slice:L ext.op.index.slice.zero:L ext.op.index.slice.bounds:L ext.op.index.slice.unsupported:L ext.op.index.slice.assign:L ext.op.index.slice.length:L ext.op.index.slice.detached:L ext.op.comprehension.async:L ext.op.comprehension.async.unavailable:L ext.op.comprehension.target.unavailable:L ext.builtin.sum.non_number:L ext.builtin.range.non_integer:L ext.builtin.range.zero_step:L ext.op.comprehension.for:L ext.op.comprehension.in:L ext.op.comprehension.if:L ext.syntax.set:B ext.syntax.array.spread:L ext.syntax.map.spread:L ext.syntax.collection.unwalkable:L ext.syntax.map.spread.unmapped:L ext.op.comprehension.unpack.amiss:L ext.builtin.range.value:B ext.builtin.sum:L ext.builtin.list:L ext.builtin.any:L ext.op.await:L ext.stmt.async:L ext.stmt.loop.else:B ext.stmt.del.unrun:L ext.stmt.binding.unrun:L ext.stmt.type_params.close:L ext.stmt.type_params.open:L ext.stmt.for.target.unready:L ext.op.identity.unready:L ext.op.identity.negated:L ext.op.identity:L ext.op.in.unready:L ext.op.in.negated:L ext.op.in:L ext.stmt.async.unready:L ext.stmt.with.unready:L ext.op.lambda.unready:L ext.op.member.pipes:B ext.op.tuple.unready:L ext.lexical.number.imaginary.unready:L ext.lexical.number.imaginary:L ext.lexical.number.point.bare:B ext.lexical.string.amiss:L ext.lexical.line_continuation.amiss:L ext.lexical.string.prefix.raw:L ext.lexical.string.prefix.plain:L ext.lexical.string.prefix.bytes:L ext.lexical.string.prefix.format:L ext.lexical.string.adjacent:B ext.lexical.string.unready:L ext.lexical.line_continuation:L ext.op.bit.whole:B ext.lexical.escape.continued:B ext.stmt.with.unrun:L ext.stmt.async.unrun:L \
ext.lexical.epilogue:L ext.builtin.echo:L ext.syntax.call.bare:B ext.op.increment:L ext.op.decrement:L \
ext.lexical.interpolating_quotes:L ext.lexical.heredoc:L ext.stmt.for.c:L ext.op.assign.compound:B ext.stmt.static:L ext.stmt.global:L \
ext.stmt.import:L ext.stmt.import.from:L ext.stmt.import.as:L ext.system.module.name:L \
ext.stmt.decorator:L ext.stmt.decorator.amiss:L ext.stmt.const:L ext.builtin.define:L ext.builtin.define.class_constant:L ext.builtin.var_dump:L ext.stmt.match:L ext.stmt.match.case:L ext.stmt.match.wildcard:L ext.stmt.match.or:L ext.stmt.match.guard:L ext.stmt.match.as:L ext.stmt.match.unready:L ext.stmt.match.invalid:L ext.stmt.switch:L ext.stmt.case:L \
ext.stmt.default:L ext.stmt.case.mark:L ext.op.ternary:L ext.block.lone_statement:B ext.stmt.function.hoisted:B ext.stmt.function.outermost:B ext.system.request.amiss:L ext.system.request.amiss.boundary:L ext.system.request.amiss.boundary.wrong:L ext.system.request.amiss.part:L ext.system.request.amiss.body.large:L ext.system.request.body:L \
ext.op.if_else:L ext.op.lambda.unsupported:L ext.op.lambda.enclosing:L ext.op.identical.negated:L ext.op.identical.unsupported:L ext.op.in:L ext.op.in.negated:L ext.op.in.unsupported:L ext.op.compare.chained:B ext.op.assign.expression:L ext.literal.ellipsis:L ext.op.rem.formats_text:B ext.op.rem.format.unsupported:L ext.op.rem.format.arguments:L ext.lexical.number.exponent:L ext.op.plus:L ext.stmt.break.levels:B ext.builtin.array:L ext.op.index.append:B ext.stmt.for.collection:B ext.builtin.print_r:L ext.stmt.terminator:L ext.stmt.annotation:L ext.stmt.annotation.amiss:L ext.stmt.annotation.target.unready:L ext.stmt.function.returns:L ext.stmt.class:L ext.stmt.class.extends:L ext.stmt.class.new:L ext.stmt.class.this:L ext.stmt.class.this.explicit:B ext.stmt.class.bases.open:L ext.stmt.class.bases.close:L ext.op.member.pipes:B ext.stmt.class.unready:L ext.lexical.line_continuation:L ext.lexical.number.point.bare:B ext.lexical.number.separator.after_prefix:B ext.op.bit.whole:B ext.builtin.print.real_point:B ext.lexical.number.imaginary:L ext.lexical.number.imaginary.unready:L ext.lexical.number.point_edge:B ext.lexical.number.separator.strict:B ext.lexical.number.amiss.leading_zero:L ext.lexical.number.amiss.binary:L ext.lexical.number.amiss.binary.digit:L ext.lexical.number.amiss.octal:L ext.lexical.number.amiss.octal.digit:L ext.lexical.number.amiss.hex:L ext.stmt.assign.names.chained:B ext.syntax.call.chained:B ext.op.lambda:L ext.op.lambda.unready:L ext.literal.ellipsis.unready:L ext.stmt.with:L ext.stmt.with.as:L ext.stmt.with.unready:L ext.op.tuple:L ext.op.tuple.unready:L ext.op.matrix:L ext.op.matrix.unready:L ext.stmt.loop.else:B ext.lexical.string.adjacent:B ext.lexical.string.prefix.bytes:L ext.lexical.string.bytes.unready:L ext.lexical.number.imaginary.unrun:L \
ext.stmt.class.constructor:L ext.stmt.class.modifier:L ext.stmt.class.hidden:L ext.stmt.class.guarded:L ext.stmt.class.shared:L ext.op.member:L ext.op.scope:L \
ext.op.instanceof:L ext.stmt.class.parent:L ext.stmt.class.self:L ext.lexical.name_lead:L ext.stmt.assert:L ext.stmt.assert.kind:L ext.stmt.catch.invalid:L ext.stmt.catch.as:L ext.stmt.catch.tuple.open:L ext.stmt.catch.tuple.close:L ext.stmt.catch.group:L ext.stmt.catch.group.unsupported:L ext.stmt.try.else:B ext.stmt.throw.from:L ext.stmt.throw.empty:L ext.stmt.try:L \
ext.stmt.catch:L ext.stmt.finally:L ext.stmt.throw:L ext.stmt.catch.separator:L ext.op.reference:L \
ext.system.request.query:L ext.system.request.form:L ext.system.request.cookies:L ext.system.request.server:L \
ext.system.request.env:L ext.system.request.files:L ext.system.request.all:L ext.system.request.settings:L ext.op.index.absent:B ext.op.index.scalar:L ext.op.index.nothing:L ext.stmt.class.destructor:L ext.stmt.class.reader:L ext.stmt.class.writer:L ext.stmt.class.caller:L \
ext.system.args.list:L ext.system.args.count:L ext.op.walk.class:L ext.op.walk.rewind:L ext.op.walk.more:L ext.op.walk.this:L ext.op.walk.key:L \
ext.op.walk.onward:L ext.op.walk.giver.class:L ext.op.walk.giver:L ext.op.walk.no_cell:L ext.op.walk.key.no_cell:L ext.op.walk.live:B ext.builtin.array.front:L ext.stmt.case.mark.instead:L \
ext.stmt.function.carries.pairs:L ext.stmt.function.keyword_only:L ext.stmt.function.positional_only:L ext.syntax.call.bind_names:B ext.syntax.call.spread:L ext.syntax.call.spread.pairs:L ext.syntax.call.amiss:L ext.syntax.call.amiss.missing:L ext.syntax.call.amiss.unknown:L ext.syntax.call.amiss.duplicate:L ext.syntax.call.amiss.builtin:L ext.builtin.print.sep:L ext.builtin.print.end:L ext.builtin.print.file:L ext.builtin.print.flush:L ext.builtin.print.file.error:L ext.builtin.print.file.output:L ext.builtin.print.file.unready:L ext.builtin.print.sep.amiss:L ext.builtin.print.end.amiss:L ext.builtin.to_int.base:L ext.builtin.to_int.base.amiss:L ext.builtin.to_int.text.amiss:L ext.builtin.to_int.text.required:L ext.builtin.to_real.text:B ext.builtin.to_real.text.amiss:L ext.builtin.to_string.object:L ext.builtin.to_string.encoding:L ext.builtin.to_string.errors:L ext.builtin.to_string.unready:L ext.builtin.range.value:B ext.builtin.range.zero:L ext.builtin.range.integer:L ext.builtin.range.index:L ext.syntax.call.spread.amiss:L ext.syntax.call.spread.pairs.amiss:L ext.stmt.function.defaults.amiss:L ext.stmt.function.parameters.amiss:L ext.stmt.function.carries:L ext.stmt.function.short:L ext.stmt.class.trait:L ext.stmt.class.uses:L ext.stmt.class.uses.alias:L ext.stmt.class.interface:L ext.stmt.class.implements:L ext.op.compare:L ext.builtin.unset:L ext.lexical.template:B ext.lexical.prologue.echo:L ext.lexical.prologue.folded:B ext.lexical.line_continuation:L ext.lexical.number.separator.after_prefix:B ext.op.bit.whole:B ext.op.bit.whole.room:L ext.op.plus.non_number:L \
ext.op.otherwise:L ext.op.bit.and:L ext.op.bit.or:L ext.op.bit.xor:L ext.op.bit.not:L \
ext.op.bit.left:L ext.op.bit.right:L ext.op.bit.shift.numbers:B ext.op.identical:L ext.op.not_identical:L ext.system.kind.spelled:B ext.builtin.args.all:L \
ext.builtin.args.count:L ext.builtin.args.at:L ext.builtin.args.all.outside:L ext.builtin.args.count.outside:L ext.builtin.args.at.outside:L ext.builtin.args.at.below:L ext.builtin.args.at.beyond:L ext.op.assign.value:B ext.op.index.plain_keys:B \
ext.system.runner:L ext.system.source.file:L ext.system.source.directory:L ext.system.source.line:L ext.system.source.routine:L ext.system.source.class:L ext.system.source.method:L \
ext.system.complaint.warning:L ext.system.complaint.notice:L ext.system.complaint.deprecated:L ext.system.complaint.fatal:L ext.system.complaint.reading:L ext.system.fault.class:L ext.system.fault.operands:L ext.op.increment.text:L ext.op.decrement.text:L ext.system.fault.class.arithmetic:L ext.system.fault.class.division:L ext.system.fault.class.kind:L ext.system.fault.class.value:L ext.system.fault.modulo:L ext.system.fault.shift:L ext.system.fault.class.walk:L ext.op.walk.giver.unwalkable:L ext.builtin.time_limit:L ext.system.kind.brief:L ext.builtin.file.read:L ext.builtin.file.write:L \
ext.builtin.file.exists:L ext.builtin.file.remove:L ext.builtin.eval:L ext.builtin.include:L ext.builtin.include.once:L ext.builtin.output.hold:L ext.builtin.output.held:L ext.builtin.output.drop:L ext.builtin.output.depth:L ext.builtin.output.begun:L ext.builtin.at_end:L ext.builtin.complaint.handler:L ext.builtin.complaint.say:L ext.builtin.calls:L ext.system.kind.object:L ext.system.kind.loose:L ext.builtin.uncaught:L ext.builtin.classes:L ext.builtin.routines:L ext.builtin.spelled:L ext.builtin.class.beneath:L ext.builtin.math:L ext.builtin.class.methods:L ext.builtin.class.properties:L ext.builtin.clock:L ext.builtin.room.used:L ext.builtin.room.most:L ext.builtin.room.most.forget:L ext.builtin.room.limit:L ext.builtin.write.operator:B ext.op.hush:L ext.op.name_by_value:L ext.op.cast:B ext.op.member.by_value:B ext.op.index.text:B ext.op.index.text.first:L ext.system.globals:L ext.op.reference.unshared.written:L ext.op.reference.unshared.given:L ext.op.reference.unshared.handed:L ext.stmt.terminator.only:B ext.stmt.block.instead:L ext.stmt.block.instead.close:L ext.op.spelled:B ext.system.class.folded:B ext.stmt.unpack:L ext.builtin.isset:L ext.builtin.empty:L ext.stmt.do:L ext.op.index.makes:B ext.system.untrue.text:L ext.system.untrue.empty_array:B ext.builtin.exit:L ext.lexical.string.long:L ext.lexical.string.prefix.raw:L ext.lexical.string.prefix.bytes:L ext.lexical.string.prefix.plain:L ext.lexical.string.prefix.format:L ext.lexical.string.adjacent:B ext.lexical.string.amiss:L ext.lexical.escape.byte.digits:N ext.lexical.escape.codepoint.digits:N ext.lexical.escape.codepoint.wide:L ext.lexical.escape.codepoint.wide.digits:N ext.lexical.escape.named:L ext.lexical.escape.unavailable:L ext.lexical.escape.continued:B ext.lexical.escape.controls:L ext.lexical.escape.codepoint:L ext.lexical.escape.codepoint.open:L ext.lexical.escape.codepoint.close:L ext.lexical.escape.codepoint.amiss:L ext.lexical.escape.codepoint.beyond:L ext.lexical.number.amiss:L ext.stmt.separator:L ext.op.tuple:L ext.stmt.unpack.rest:L ext.stmt.assign.chain:B ext.stmt.unpack.short:L ext.stmt.unpack.long:L ext.stmt.unpack.unwalkable:L ext.stmt.unpack.amiss:L ext.stmt.class.bases.open:L ext.stmt.class.bases.close:L ext.stmt.class.unready:L ext.op.member.pipes:B ext.stmt.with:L ext.stmt.with.as:L ext.stmt.with.unready:L ext.op.tuple.unready:L ext.op.identical.negated:L ext.op.identical.unsupported:L ext.builtin.range.zero_start:B ext.lexical.string.value.unready:L ext.lexical.line_continuation:L ext.lexical.string.bytes.unavailable:L ext.lexical.string.format.unavailable:L ext.lexical.string.unready:L \
ext.builtin.shell:L ext.builtin.wait:L ext.builtin.net.ask:L ext.builtin.run.begin:L ext.builtin.run.end:L ext.lexical.escape.byte:L ext.lexical.escape.octal:B ext.system.text.bytes:B ext.lexical.prologue.brief:L ext.lexical.prologue.brief.setting:L ext.lexical.interpolating.index.amiss:L ext.builtin.eval.place:L \
ext.system.reading.unexpected:L ext.system.reading.unexpected.character:L ext.system.fault.class.reading:L \
ext.system.reading.unclosed:L ext.system.reading.unclosed.line:L ext.system.reading.unclosed.mismatch:L ext.system.reading.unmatched:L \
ext.lexical.number.binary_prefix:L ext.lexical.number.octal_prefix:L ext.lexical.number.octal_lead:B \
ext.lexical.number.separator:L ext.system.integer.bits:N ext.system.real.bits:N ext.system.real.digits:N \
ext.system.real.figures:L ext.system.real.figures.shown:L \
ext.stmt.function.own_names:B ext.stmt.static.read_in:B \
ext.system.complaint.markup.setting:L ext.system.complaint.markup.kind:L ext.system.complaint.markup.place:L ext.system.complaint.markup.line:L ext.system.complaint.markup.reference:L \
ext.system.complaint.reference.setting:L ext.system.complaint.reference.page:L ext.system.complaint.reference.mark:L \
ext.builtin.include.demanded:L ext.builtin.include.demanded.missing:L \
 ext.stmt.with.unready:L ext.op.member.pipes:B ext.op.tuple.unready:L ext.lexical.string.prefix.bytes.unready:L ext.lexical.string.prefix.format.unready:L ext.stmt.assign.chain:B ext.lexical.escape.deferred:L ";

fn tag_shapes(table: &'static str) -> Vec<(&'static str, char)> {
    table.split_whitespace().map(|e| {
        let (k, s) = e.rsplit_once(':').unwrap();
        (k, s.chars().next().unwrap())
    }).collect()
}

fn cell_of(key: &str, shape: char, json: &Json) -> Result<Entry, String> {
    Ok(match (shape, json) {
        ('L', j) => Entry::Strings(strings_at(key, j)?),
        ('B', Json::Bool(b)) => Entry::Flag(*b),
        ('N', Json::Null) => Entry::Count(None),
        ('N', Json::Number(n)) if n.as_u64().is_some() => Entry::Count(n.as_u64().map(|n| n as usize)),
        ('W', Json::String(s)) if !s.is_empty() => Entry::One(Some(s.clone())),
        ('O', Json::Null) => Entry::One(None),
        ('O', Json::String(s)) if !s.is_empty() => Entry::One(Some(s.clone())),
        ('T', Json::Array(tiers)) => Entry::Tiers(tiers.iter().map(|t| strings_at(key, t)).collect::<Result<_, _>>()?),
        _ => return Err(format!("label '{key}' has the wrong shape")),
    })
}

/// Labels this kernel gives no meaning to: read past, as an ext.* label is.
#[allow(dead_code)]
const MUST_BE_EMPTY: [&str; 8] = [
    "syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close",
    "stmt.foreach", "stmt.foreach.as", "stmt.foreach.pair", "stmt.emit",
];

/// Builtin labels and the operation each names.
pub const BUILTIN_LABELS: [(&str, Prim); 72] = [
    ("ext.builtin.sum", Prim::Total), ("ext.builtin.list", Prim::Listed), ("ext.builtin.any", Prim::SomeTrue),
    ("builtin.emit", Prim::Echo), ("builtin.print", Prim::Say), ("builtin.write", Prim::Out), ("builtin.len", Prim::Length),
    ("builtin.char_at", Prim::CharAtIndex), ("builtin.ord", Prim::CodeOf), ("builtin.chr", Prim::CharOf), ("builtin.typeof", Prim::SortOf),
    ("builtin.error", Prim::Raise), ("builtin.extern", Prim::External), ("builtin.range", Prim::Span), ("builtin.real", Prim::MakeReal),
    ("builtin.precision", Prim::Places), ("builtin.to_string", Prim::AsText), ("builtin.to_int", Prim::AsInt),
    ("builtin.to_real", Prim::AsReal), ("builtin.num", Prim::Numer), ("builtin.den", Prim::Denom), ("builtin.push", Prim::Append),
    ("builtin.get", Prim::Fetch), ("builtin.put", Prim::Replace), ("ext.builtin.echo", Prim::Tell),
    ("ext.builtin.define", Prim::Define), ("ext.builtin.var_dump", Prim::Dump), ("ext.builtin.array", Prim::Gather),
    ("ext.builtin.print_r", Prim::Portray), ("ext.builtin.unset", Prim::Erase), ("ext.builtin.array.front", Prim::Front), ("ext.builtin.isset", Prim::Standing), ("ext.builtin.empty", Prim::Hollow), ("ext.builtin.exit", Prim::Quit),
    ("ext.builtin.args.all", Prim::Handed), ("ext.builtin.args.count", Prim::HowMany),
    ("ext.builtin.args.at", Prim::HandedAt), ("ext.builtin.time_limit", Prim::Clock),
    ("ext.builtin.eval", Prim::Weigh), ("ext.builtin.include", Prim::Bring), ("ext.builtin.include.once", Prim::BringOnce),
    ("ext.builtin.output.hold", Prim::KeepOut), ("ext.builtin.output.held", Prim::KeptOut),
    ("ext.builtin.output.drop", Prim::LooseOut), ("ext.builtin.output.depth", Prim::DeepOut),
    ("ext.builtin.output.begun", Prim::OutBegun),
    ("ext.builtin.at_end", Prim::Afterward), ("ext.builtin.complaint.handler", Prim::Hearer), ("ext.builtin.complaint.say", Prim::Complain),
    ("ext.builtin.calls", Prim::Under),
    ("ext.builtin.uncaught", Prim::Untaken),
    ("ext.builtin.classes", Prim::ClassesBound), ("ext.builtin.routines", Prim::RoutinesBound), ("ext.builtin.spelled", Prim::WordsSpelled), ("ext.builtin.class.methods", Prim::ClassMethods), ("ext.builtin.class.properties", Prim::ClassProperties),
    ("ext.builtin.class.beneath", Prim::ClassBeneath), ("ext.builtin.math", Prim::Reckon),
    ("ext.builtin.clock", Prim::SinceEpoch),
    ("ext.builtin.room.used", Prim::RoomHeld), ("ext.builtin.room.most", Prim::RoomHighest),
    ("ext.builtin.room.most.forget", Prim::RoomAnew), ("ext.builtin.room.limit", Prim::RoomMark),
    ("ext.builtin.file.read", Prim::Slurp), ("ext.builtin.file.write", Prim::Spill),
    ("ext.builtin.file.exists", Prim::There), ("ext.builtin.file.remove", Prim::Gone),
    ("ext.builtin.shell", Prim::Shelled), ("ext.builtin.net.ask", Prim::Reached), ("ext.builtin.wait", Prim::Bided),
    ("ext.builtin.run.begin", Prim::Raised), ("ext.builtin.run.end", Prim::Laid),
];

const BINARY_LABELS: [(&str, Prim); 26] = [
    ("op.add", Prim::Plus), ("op.sub", Prim::Minus), ("op.mul", Prim::Times), ("op.div", Prim::Over), ("op.quot", Prim::IntDiv),
    ("op.rem", Prim::Mod), ("op.pow", Prim::Power), ("op.eq", Prim::Eq), ("op.ne", Prim::Ne), ("op.lt", Prim::Lt), ("op.le", Prim::Le),
    ("op.gt", Prim::Gt), ("op.ge", Prim::Ge), ("op.and", Prim::Both), ("op.or", Prim::Either), ("op.concat", Prim::Join),
    ("ext.op.matrix", Prim::MatrixProduct), ("ext.op.compare", Prim::Rank), ("ext.op.bit.and", Prim::BitsBoth), ("ext.op.bit.or", Prim::BitsEither),
    ("ext.op.bit.xor", Prim::BitsOne), ("ext.op.bit.left", Prim::BitsUp), ("ext.op.bit.right", Prim::BitsDown),
    ("ext.op.in", Prim::Contains), ("ext.op.identical", Prim::Selfsame), ("ext.op.not_identical", Prim::Unlike),
];

fn top_object(text: &str) -> Result<serde_json::Map<String, Json>, String> {
    match serde_json::from_str(text).map_err(|e| format!("definition is not valid JSON: {e}"))? {
        Json::Object(m) => Ok(m),
        _ => Err("definition must be a JSON object".to_string()),
    }
}

pub fn identify(text: &str) -> Result<(String, Vec<String>), String> {
    let m = top_object(text)?;
    let name = m.get("language").and_then(Json::as_str).filter(|s| !s.is_empty()).ok_or("label 'language' must be a non-empty string")?;
    let exts = m.get("extensions").and_then(Json::as_array).ok_or("label 'extensions' must be a list of strings")?;
    Ok((name.to_string(), exts.iter().filter_map(|e| e.as_str().map(str::to_string)).collect()))
}

fn strings_at(key: &str, json: &Json) -> Result<Vec<String>, String> {
    let Json::Array(items) = json else { return Err(format!("label '{key}' must be a list of non-empty strings")) };
    items
        .iter()
        .map(|i| match i.as_str() {
            Some(s) if !s.is_empty() => Ok(s.to_string()),
            _ => Err(format!("label '{key}' must be a list of non-empty strings")),
        })
        .collect()
}

impl Table {
    pub fn parse(text: &str) -> Result<Table, String> {
        let map = top_object(text)?;
        let mut cells = HashMap::new();
        let mut known = tag_shapes(TAGS);
        for (key, shape) in &known {
            let json = map.get(*key).ok_or_else(|| format!("missing label '{key}'"))?;
            cells.insert(*key, cell_of(key, *shape, json)?);
        }
        for (key, shape) in tag_shapes(EXT_TAGS) {
            let cell = match map.get(key) {
                Some(json) => cell_of(key, shape, json)?,
                None if shape == 'B' => Entry::Flag(false),
                None => Entry::Strings(Vec::new()),
            };
            cells.insert(key, cell);
            known.push((key, shape));
        }
        let mut strange: Vec<&str> = map.keys().map(String::as_str).filter(|k| !k.starts_with('$') && !known.iter().any(|(l, _)| l == k)).collect();
        strange.sort();
        if !strange.is_empty() {
            return Err(format!("unknown label(s): {}", strange.join(", ")));
        }
        let mut table = Table {
            ident: String::new(),
            blocks: Blocks::Indented,
            rpn: false,
            cells,
            dyadic: HashMap::new(),
            monadic: HashMap::new(),
            precedence: HashMap::new(),
            prims: HashMap::new(),
            compound: HashMap::new(),
            given: map.keys().cloned().collect(),
            keywords: HashSet::new(),
            signs: Vec::new(),
        };
        table.ident = table.lone("language").unwrap_or("").to_string();
        table.check()?;
        let extra_ends = table.strings("ext.stmt.terminator").to_vec();
        if let Some(Entry::Strings(ends)) = table.cells.get_mut("stmt.terminator") {
            ends.extend(extra_ends);
        }
        table.precedence_tables()?;
        Ok(table)
    }

    pub fn strings(&self, key: &str) -> &[String] {
        match self.cells.get(key) {
            Some(Entry::Strings(l)) => l,
            _ => panic!("'{key}' is not a list label"),
        }
    }

    /// Empty a label of words, so that everything reading it afterwards
    /// finds none. The label is left standing rather than taken away,
    /// since a label the roster names must always be there to be read.
    /// It is for a label whose worth a run is started with rather than
    /// written into the definition.
    pub fn put_by(&mut self, key: &'static str) {
        self.cells.insert(key, Entry::Strings(Vec::new()));
    }

    /// A required ending or an optional parting between statements.
    pub fn separates(&self, sign: &str) -> bool {
        self.spells("stmt.terminator", sign) || self.spells("ext.stmt.separator", sign)
    }

    pub fn single(&self, key: &str) -> Option<&str> {
        self.strings(key).first().map(String::as_str)
    }

    /// A label whose words stand on either side of what the kernel
    /// writes between them: the first word before, the second after.
    pub fn around(&self, key: &str) -> Option<(&str, &str)> {
        match self.strings(key) {
            [before, after, ..] => Some((before.as_str(), after.as_str())),
            _ => None,
        }
    }

    /// Whether the definition gave this label at all, even empty: an
    /// empty list can still say something.
    pub fn cell_present(&self, key: &str) -> bool {
        self.given.contains(key)
    }

    pub fn has_any(&self, key: &str) -> bool {
        !self.strings(key).is_empty()
    }

    pub fn spells(&self, key: &str, lexeme: &str) -> bool {
        self.strings(key).iter().any(|w| w == lexeme)
    }

    pub fn flag(&self, key: &str) -> bool {
        matches!(self.cells.get(key), Some(Entry::Flag(true)))
    }

    pub fn count(&self, key: &str) -> Option<usize> {
        match self.cells.get(key) {
            Some(Entry::Count(n)) => *n,
            _ => None,
        }
    }

    pub fn lone(&self, key: &str) -> Option<&str> {
        match self.cells.get(key) {
            Some(Entry::One(w)) => w.as_deref(),
            _ => None,
        }
    }

    fn tiers(&self) -> &[Vec<String>] {
        match self.cells.get("op.precedence") {
            Some(Entry::Tiers(t)) => t,
            _ => &[],
        }
    }

    pub fn letter(&self, key: &str) -> Option<char> {
        let mut it = self.single(key)?.chars();
        match (it.next(), it.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        }
    }

    pub fn letters(&self, key: &str) -> Vec<char> {
        self.strings(key).iter().filter_map(|w| if w.chars().count() == 1 { w.chars().next() } else { None }).collect()
    }

    pub fn banner(&self) -> String {
        let mut it = self.ident.chars();
        it.next().map_or("Error".to_string(), |c| format!("{}{}Error", c.to_uppercase(), it.as_str()))
    }

    pub fn begins_name(&self, c: char) -> bool {
        // A language keeping text as bytes spells its names in bytes as
        // well, so any byte above the plain seven-bit ones may stand in
        // one, whatever letter it would otherwise be part of.
        let a_byte = self.flag("ext.system.text.bytes");
        let far = a_byte && self.flag("identifier.unicode") && c >= '\u{80}';
        if a_byte {
            return c == '_' || c.is_ascii_alphabetic() || far;
        }
        c == '_' || if self.flag("identifier.unicode") { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
    }

    pub fn extends_name(&self, c: char) -> bool {
        let a_byte = self.flag("ext.system.text.bytes");
        let far = a_byte && self.flag("identifier.unicode") && c >= '\u{80}';
        if a_byte {
            return c == '_' || c.is_ascii_alphanumeric() || far;
        }
        c == '_' || if self.flag("identifier.unicode") { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
    }

    /// What a piece of text comes to as bytes. Where the language keeps
    /// text as bytes every character is worth one, its own number over
    /// again; elsewhere the bytes are those the letters are written in.
    pub fn raw_of(&self, s: &str) -> Vec<u8> {
        if !self.flag("ext.system.text.bytes") {
            return s.as_bytes().to_vec();
        }
        s.chars().map(|c| c as u32 as u8).collect()
    }

    /// And a run of bytes as the text it comes to, the same road back.
    pub fn said_of(&self, raw: &[u8]) -> String {
        if !self.flag("ext.system.text.bytes") {
            return String::from_utf8_lossy(raw).into_owned();
        }
        raw.iter().map(|b| char::from(*b)).collect()
    }

    pub fn name_like(&self, s: &str) -> bool {
        let mut it = s.chars();
        let mut first = it.next();
        if first.is_some() && first == self.letter("identifier.variable_prefix") {
            first = it.next();
        }
        first.map_or(false, |c| self.begins_name(c)) && it.all(|c| self.extends_name(c))
    }

    fn check(&mut self) -> Result<(), String> {
        if self.count("format_version") != Some(1) {
            return Err("format_version must be 1".to_string());
        }
        let singles = ["lexical.string_quotes", "lexical.raw_quotes", "lexical.string_escapes", "lexical.name_quote",
            "lexical.number.decimal_point", "lexical.number.base_marker", "lexical.number.exponent_marker", "identifier.variable_prefix",
            "ext.lexical.number.exponent", "ext.lexical.interpolating_quotes"];
        for key in singles {
            if let Some(w) = self.strings(key).iter().find(|w| w.chars().count() != 1) {
                return Err(format!("label '{key}' takes single characters, got '{w}'"));
            }
        }
        let woven = self.strings("ext.lexical.interpolating_quotes");
        if let Some(q) = woven.iter().find(|q| !self.spells("lexical.string_quotes", q)) {
            return Err(format!("ext.lexical.interpolating_quotes '{q}' is not among lexical.string_quotes"));
        }
        if !woven.is_empty() && (!self.has_any("syntax.group.open") || !self.has_any("op.concat")) {
            return Err("ext.lexical.interpolating_quotes needs syntax.group and op.concat".to_string());
        }
        if self.flag("ext.syntax.call.bare") && !self.has_any("syntax.call.open") {
            return Err("ext.syntax.call.bare needs syntax.call".to_string());
        }
        if self.strings("lexical.comment_block.open").len() != self.strings("lexical.comment_block.close").len() {
            return Err("lexical.comment_block.open and .close must pair up position by position".to_string());
        }
        if let Some(p) = self.single("lexical.number.hex_prefix") {
            if p.chars().count() != 2 || !p.starts_with(|c: char| c.is_ascii_digit()) {
                return Err(format!("lexical.number.hex_prefix must be a digit followed by one letter, got '{p}'"));
            }
        }
        self.blocks = match self.lone("block.style") {
            Some("indentation") => Blocks::Indented,
            Some("braces") => Blocks::Bracketed,
            Some("keyword") => Blocks::Worded,
            other => return Err(format!("block.style must be 'indentation', 'braces' or 'keyword', got '{}'", other.unwrap_or(""))),
        };
        self.rpn = match self.lone("syntax.notation") {
            Some("infix") => false,
            Some("postfix") => true,
            other => return Err(format!("syntax.notation must be 'infix' or 'postfix', got '{}'", other.unwrap_or(""))),
        };
        let (o, c) = (self.strings("block.open").len(), self.strings("block.close").len());
        let fits = match self.blocks {
            Blocks::Indented => o == 0 && c == 0 && self.count("block.indent_size").map_or(false, |n| n > 0),
            Blocks::Bracketed => o > 0 && o == c,
            Blocks::Worded => o == 0 && c > 0,
        };
        if !fits {
            return Err("block.open, block.close and block.indent_size do not fit block.style".to_string());
        }
        let postfix = self.rpn;
        if postfix && !self.tiers().is_empty() {
            return Err("a postfix language takes no op.precedence".to_string());
        }
        let stack_labels = ["stack.dup", "stack.drop", "stack.swap", "stack.over", "stack.rot", "stack.eval", "stack.program.open", "stack.program.close"];
        if !postfix && stack_labels.iter().any(|l| self.has_any(l)) {
            return Err("the stack.* labels need syntax.notation 'postfix'".to_string());
        }
        for (a, b) in [("syntax.group.open", "syntax.group.close"), ("syntax.call.open", "syntax.call.close"),
                       ("syntax.array.open", "syntax.array.close"), ("op.index.open", "op.index.close"),
                       ("stack.program.open", "stack.program.close")] {
            if self.strings(a).len() != self.strings(b).len() && (self.strings(a).is_empty() || self.strings(b).is_empty()) {
                return Err(format!("labels '{a}' and '{b}' must be given together"));
            }
        }
        if self.has_any("syntax.call.label") && !self.has_any("syntax.call.open") {
            return Err("syntax.call.label needs syntax.call.open".to_string());
        }
        if self.flag("op.index.strings") && !self.has_any("op.index.open") {
            return Err("op.index.strings needs op.index.open".to_string());
        }
        if !matches!(self.lone("op.div.result"), None | Some("rational") | Some("real") | Some("whole_or_real")) {
            return Err("op.div.result must be 'rational', 'real', 'whole_or_real' or null".to_string());
        }
        if !self.has_any("stmt.let") && (self.has_any("stmt.let.mutable") || self.has_any("stmt.let.annotation") || self.flag("stmt.let.type_first")) {
            return Err("stmt.let.mutable, stmt.let.annotation and stmt.let.type_first need stmt.let".to_string());
        }
        if self.flag("stmt.let.type_first") {
            for l in ["stmt.let.mutable", "stmt.let.annotation", "stmt.function", "stmt.function.returns"] {
                if self.has_any(l) {
                    return Err(format!("stmt.let.type_first leaves no place for {l}; leave it empty"));
                }
            }
        }
        if !self.has_any("stmt.if") && (self.has_any("stmt.elif") || self.has_any("stmt.else")) {
            return Err("stmt.elif and stmt.else need stmt.if".to_string());
        }
        if self.has_any("stmt.for") != self.has_any("stmt.for.in") && !(postfix && !self.has_any("stmt.for.in")) {
            return Err("stmt.for and stmt.for.in must be given together".to_string());
        }
        if self.flag("stmt.function.result_by_name") && !self.has_any("stmt.function") {
            return Err("stmt.function.result_by_name needs stmt.function".to_string());
        }
        let keywords = ["stmt.let", "stmt.let.mutable", "stmt.if", "stmt.elif", "stmt.else", "stmt.while", "stmt.until", "stmt.for",
            "stmt.for.in", "stmt.return", "stmt.break", "stmt.continue", "stmt.function", "stmt.pass", "literal.true", "literal.false", "literal.null"];
        for key in keywords {
            if let Some(w) = self.strings(key).iter().find(|w| !self.name_like(w)) {
                return Err(format!("keyword '{w}' must be shaped like an identifier"));
            }
        }
        for key in ["system.args", "system.memoization", "system.real_default_precision", "system.entry", "system.kind.integer",
                    "system.kind.rational", "system.kind.real", "system.kind.string", "system.kind.boolean", "system.kind.array", "system.kind.null"] {
            if let Some(w) = self.single(key).filter(|w| !self.name_like(w)) {
                return Err(format!("system name '{w}' must be shaped like an identifier"));
            }
        }
        Ok(())
    }

    fn precedence_tables(&mut self) -> Result<(), String> {
        let postfix = self.rpn;
        let table = self.tiers().to_vec();
        let place = |lex: &str, last: bool| -> Option<u32> {
            if postfix {
                return Some(1);
            }
            let at = if last { table.iter().rposition(|t| t.contains(&lex.to_string())) } else { table.iter().position(|t| t.contains(&lex.to_string())) };
            at.map(|i| i as u32 + 1)
        };
        let real_division = matches!(self.lone("op.div.result"), Some("real") | Some("whole_or_real"));
        for (label, op) in BINARY_LABELS {
            let op = if label == "op.div" && real_division { Prim::OverReal } else { op };
            for lex in self.strings(label).to_vec() {
                let tier = place(&lex, false).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                let right = self.spells("op.right_associative", &lex);
                if self.dyadic.insert(lex.clone(), Infix { prim: op, level: tier, right_assoc: right }).is_some() {
                    return Err(format!("'{lex}' is listed under two binary operator labels"));
                }
            }
        }
        for (label, op) in [("op.not", Prim::Invert), ("op.negate", Prim::Negate), ("ext.op.bit.not", Prim::BitsOver)] {
            for lex in self.strings(label).to_vec() {
                let tier = place(&lex, true).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                if self.monadic.insert(lex.clone(), Infix { prim: op, level: tier, right_assoc: false }).is_some() {
                    return Err(format!("'{lex}' is listed under two unary operator labels"));
                }
            }
        }
        if self.flag("ext.op.assign.compound") {
            // An operator directly followed by the assignment sign, where
            // that is not itself an operator (`<=`) and the operator does
            // not end in the sign (`==`, so `===` stays an operator).
            for assign in self.strings("stmt.assign").to_vec() {
                for (lex, op) in &self.dyadic {
                    let spelled = format!("{lex}{assign}");
                    if !lex.ends_with(assign.as_str()) && !self.dyadic.contains_key(&spelled) && !self.monadic.contains_key(&spelled) {
                        self.compound.insert(spelled, op.prim);
                    }
                }
            }
        }
        match self.strings("ext.op.ternary").len() {
            0 | 2 => {}
            _ => return Err("ext.op.ternary takes exactly two signs, the question and the mark".to_string()),
        }
        if self.has_any("ext.stmt.switch") && (!self.has_any("ext.stmt.case") || !self.has_any("ext.stmt.case.mark")) {
            return Err("ext.stmt.switch needs ext.stmt.case and ext.stmt.case.mark".to_string());
        }
        let type_open = self.has_any("ext.stmt.type_params.open");
        if type_open != self.has_any("ext.stmt.type_params.close") {
            return Err("ext.stmt.type_params needs both opening and closing brackets".into());
        }
        if self.has_any("ext.stmt.class") && (!self.has_any("ext.op.member") || (!self.has_any("ext.stmt.class.new") && !self.flag("ext.stmt.class.this.explicit"))) {
            return Err("ext.stmt.class needs ext.op.member and ext.stmt.class.new".to_string());
        }
        if self.has_any("stmt.foreach") && !self.has_any("stmt.foreach.as") {
            return Err("stmt.foreach needs stmt.foreach.as".to_string());
        }
        if self.has_any("stmt.foreach.pair") && !self.has_any("syntax.map.pair") {
            return Err("stmt.foreach.pair needs syntax.map.pair".to_string());
        }
        for label in ["op.range", "op.pipe", "ext.op.hush"] {
            for lex in self.strings(label).to_vec() {
                let tier = place(&lex, false).ok_or_else(|| format!("'{lex}' ({label}) does not appear in op.precedence"))?;
                self.precedence.insert(lex, tier);
            }
        }
        for lex in table.iter().flatten() {
            if !self.dyadic.contains_key(lex) && !self.monadic.contains_key(lex) && !self.precedence.contains_key(lex) {
                return Err(format!("op.precedence lists '{lex}', which is under no operator label"));
            }
        }
        for lex in self.strings("op.right_associative") {
            if !self.dyadic.contains_key(lex) {
                return Err(format!("op.right_associative lists '{lex}', which is not a binary operator"));
            }
        }
        let quotes = self.letters("lexical.string_quotes");
        for (label, op) in BUILTIN_LABELS {
            for lex in self.strings(label).to_vec() {
                let fine = lex.starts_with(|c: char| c == '_' || c.is_alphabetic()) && !lex.chars().any(|c| c.is_whitespace() || quotes.contains(&c));
                if !fine {
                    return Err(format!("builtin name '{lex}' must begin like an identifier and hold no spaces or quotes"));
                }
                if self.prims.insert(lex, op).is_some() {
                    return Err(format!("a builtin name is listed under two builtin labels"));
                }
            }
        }
        let mut all: Vec<String> = self.dyadic.keys().chain(self.monadic.keys()).chain(self.precedence.keys()).chain(self.compound.keys()).cloned().collect();
        let symbol_labels = ["ext.lexical.string.long", "ext.op.lambda", "ext.op.tuple", "ext.stmt.class.bases.open", "ext.stmt.class.bases.close", "ext.stmt.del", "ext.stmt.nonlocal", "ext.stmt.with", "ext.stmt.with.as", "ext.stmt.yield", "ext.stmt.yield.from", "ext.op.index.slice.ellipsis", "ext.op.index.slice", "ext.op.comprehension.async", "ext.op.comprehension.for", "ext.op.comprehension.in", "ext.op.comprehension.if", "ext.syntax.array.spread", "ext.syntax.map.spread", "syntax.group.open", "syntax.group.close", "syntax.call.open", "syntax.call.separator", "syntax.call.close", "ext.op.if_else", "ext.op.identical.negated", "ext.op.in.negated", "ext.op.assign.expression", "ext.literal.ellipsis", "ext.lexical.line_continuation", "ext.stmt.type_params.close", "ext.stmt.type_params.open", "ext.op.identity.negated", "ext.op.identity", "ext.op.in", "ext.op.await", "ext.stmt.async", "ext.lexical.number.imaginary",
            "syntax.call.label", "syntax.array.open", "syntax.array.separator", "syntax.array.close", "op.index.open", "op.index.close",
            "block.intro", "stmt.assign", "stmt.terminator", "ext.stmt.separator", "stmt.let.annotation", "stmt.function.returns", "stack.dup", "stack.drop",
            "stack.swap", "stack.over", "stack.rot", "stack.eval", "stack.program.open", "stack.program.close", "stmt.let",
            "stmt.let.mutable", "stmt.if", "stmt.elif", "stmt.else", "stmt.while", "stmt.until", "stmt.for", "stmt.for.in",
            "stmt.return", "stmt.break", "stmt.continue", "stmt.function", "stmt.pass", "literal.true", "literal.false", "literal.null",
            "ext.op.increment", "ext.op.decrement", "ext.stmt.case.mark", "ext.op.ternary", "ext.stmt.for.c", "ext.stmt.static",
            "ext.stmt.import", "ext.stmt.import.from", "ext.stmt.import.as", "ext.stmt.global", "ext.stmt.decorator", "ext.stmt.const", "ext.stmt.switch", "ext.stmt.case", "ext.stmt.default", "ext.op.plus", "ext.lexical.line_continuation", "ext.stmt.match.or", "ext.op.lambda", "ext.op.assign.expression", "ext.literal.ellipsis", "ext.stmt.with", "ext.stmt.with.as", "ext.op.matrix", "ext.op.tuple",
            "syntax.map.open", "syntax.map.separator", "syntax.map.pair", "syntax.map.close", "stmt.foreach", "stmt.foreach.as",
            "ext.stmt.function.carries", "ext.stmt.function.carries.pairs", "ext.stmt.function.keyword_only", "ext.stmt.function.positional_only", "ext.syntax.call.spread", "ext.syntax.call.spread.pairs", "ext.stmt.terminator", "ext.stmt.annotation", "ext.stmt.function.returns", "ext.op.member", "ext.op.scope", "ext.stmt.class", "ext.stmt.class.extends", "ext.stmt.class.new", "ext.stmt.class.modifier", "ext.stmt.class.shared", "ext.op.instanceof", "ext.stmt.class.bases.open", "ext.stmt.class.bases.close",
            "ext.stmt.class.parent", "ext.stmt.class.self", "ext.stmt.class.interface", "ext.stmt.class.implements",
            "ext.stmt.assert", "ext.stmt.catch.as", "ext.stmt.catch.tuple.open", "ext.stmt.catch.tuple.close", "ext.stmt.catch.group", "ext.stmt.throw.from", "ext.stmt.try", "ext.stmt.catch", "ext.stmt.finally", "ext.stmt.with", "ext.stmt.with.as", "ext.stmt.del", "ext.stmt.nonlocal", "ext.stmt.async", "ext.op.await", "ext.stmt.yield", "ext.stmt.yield.from", "ext.stmt.throw", "ext.stmt.catch.separator", "ext.op.reference", "ext.op.otherwise", "ext.op.hush", "ext.op.name_by_value", "ext.stmt.unpack", "ext.op.tuple", "ext.stmt.unpack.rest"];
        for key in symbol_labels {
            all.extend(self.strings(key).iter().cloned());
        }
        if self.blocks != Blocks::Indented {
            all.extend(self.strings("block.open").iter().cloned());
            all.extend(self.strings("block.close").iter().cloned());
        }
        let fold = self.flag("lexical.keywords_case_insensitive");
        for lex in all {
            if self.name_like(&lex) {
                self.keywords.insert(if fold { lex.to_lowercase() } else { lex });
            } else if !self.signs.contains(&lex) {
                self.signs.push(lex);
            }
        }
        self.signs.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
        Ok(())
    }
}
