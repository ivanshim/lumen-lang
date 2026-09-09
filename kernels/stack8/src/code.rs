// The eight words. A routine is a list of them; the engine runs the list
// over one data stack. Five are the floor of stack5: a constant, a read,
// a write, an action on the stack's top, a conditional skip. Three are
// fused from runs of those, where the kernel lab measured that a word
// earns its place: an operator whose operands come straight from cells,
// a cell stepped in place, and a comparison with its skip.

use std::rc::Rc;

/// What is put after a class's name to file it under: a class and a
/// routine may go by one name, and a binding is one thing, so the three
/// are told apart by how each is filed rather than by what it holds.
pub const OF_A_CLASS: &str = "\0class";

use crate::value::Value;

/// A watched body and its arms, as spans in the same routine. Keeping
/// them in that routine leaves their bindings and outward leaps whole.
#[derive(Debug, Clone)]
pub struct Attempt {
    pub context: Option<Cell>,
    pub body: (usize, usize),
    pub clauses: Vec<Taking>,
    pub otherwise: Option<(usize, usize)>,
    pub last: Option<(usize, usize)>,
    pub after: usize,
}

#[derive(Debug, Clone)]
pub struct Taking {
    pub kinds: Vec<(usize, usize)>,
    pub held: Option<Cell>,
    pub body: (usize, usize),
    pub grouped: bool,
    pub bare: bool,
}

impl Attempt {
    pub fn move_marks(&mut self, mut at: impl FnMut(usize) -> usize) {
        let span = |pair: &mut (usize, usize), at: &mut dyn FnMut(usize) -> usize| {
            pair.0 = at(pair.0);
            pair.1 = at(pair.1);
        };
        span(&mut self.body, &mut at);
        for clause in &mut self.clauses {
            for kind in &mut clause.kinds { span(kind, &mut at); }
            span(&mut clause.body, &mut at);
        }
        for pair in [&mut self.otherwise, &mut self.last].into_iter().flatten() {
            span(pair, &mut at);
        }
        self.after = at(self.after);
    }
}

/// A binding's address: candidate local slots (innermost first) and the
/// global slot of the same name. A load reads the first local that holds
/// a value and falls through to the global; a store writes the first
/// local, or the global when there is none.
#[derive(Debug, Clone)]
pub struct Cell {
    pub ident: Rc<str>,
    pub near: Vec<usize>,
    pub far: usize,
    /// A load that moves the value out and leaves a hole, so the value
    /// is unshared while an operation rewrites it; the next store to the
    /// same name fills the hole.
    pub moving: bool,
}

/// Kernel operations a language can spell. `Apply` names one of these.
#[derive(Debug, Clone)]
pub enum Action {
    ContextEnter,
    SettleObjects,
    Add,
    /// A step onward or back (`++`, `--`), which is adding or taking
    /// away one save where a language steps text along its letters.
    Step(bool),
    Sub,
    Mul,
    Div,
    DivReal,
    IntDiv,
    Mod,
    Power,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Join,
    /// A field rendered with its specification and conversion.
    StringRender,
    /// Text whose reading succeeded but whose value cannot be held.
    StringFault,
    At,
    /// The three bounds of a span, kept until its array is known.
    Slice,
    /// A span whose meaning the run cannot yet honour.
    SliceUnavailable,
    Not,
    Negate,
    /// Whether a value is true, as a boolean.
    AsBool,
    /// Run the program on top with the arguments under it; a function
    /// pushes its result, a postfix program leaves what it pushed.
    Invoke(Rc<str>),
    /// Run the program on top with no arguments.
    Evaluate,
    /// Run the value on top if it is a program; push it back otherwise.
    Execute,
    /// The arguments as an array.
    MakeArray,
    /// A literal grows by one item, or by all the items of a spread.
    GatherItem { map: bool, spread: bool },
    /// The values walked by a comprehension, with maps handing out keys.
    ComprehensionItems,
    UnpackCount(usize),
    /// A map from the values above: every tie a pair, everything else
    /// keyed by its position among the untied.
    MakeMap,
    /// `k => v` as one value, until a literal gathers it.
    Tie,
    /// The key, or the value, of the pair at a position: how a foreach
    /// walks an array or a map.
    KeyAt,
    ValueAt,
    /// What a value holds at that place, read while the value is being
    /// taken apart. The same as the above, save that a value with no
    /// places at all is spoken of in the words a taking-apart uses.
    Apart,
    /// Check the extent, gathering the starred place before writes begin.
    Unpack(usize, Option<usize>),
    /// Join the gathered portions of a tuple.
    TupleJoin,
    /// What a value holds at that place, read so that what comes of it
    /// may be written back there. A value with no places at all is no
    /// place to write, so it is refused as a write to one is.
    Toward,
    /// `a[]`, which only an assignment may write to.
    AtEnd,
    /// The value under the thing it is about to be written into, made
    /// what a place in that thing will hold. Where the thing is text
    /// and the language writes into text, a place holds one letter, so
    /// only the first of what was handed over goes in and that letter
    /// is what the write itself is worth; anything else is left as it
    /// stands. The thing is handed back untouched, the write still
    /// wanting it.
    Fitted,
    /// How many places an array or a map holds.
    Extent,
    /// Whether the place a walk has reached holds a member the walk may
    /// hand out. A property taken off a thing leaves its place behind,
    /// and one the class keeps to itself is no business of a walk
    /// written outside it: either is stepped over. The thing walked is
    /// below the place, and the class the walk is written in, if any, is
    /// carried here.
    Standing(Option<Rc<str>>),
    /// What a walk walks. A thing that hands another over to be walked
    /// in its stead answers with that one; a thing that is its own walk
    /// is wound back and answers with itself; anything else is itself.
    WalkFrom,
    /// Whether a walk has more to hand out, what stands at the place it
    /// has reached and what that is called, and the step onward. A thing
    /// that is its own walk is asked; anything else is counted through,
    /// as an array is.
    WalkMore,
    WalkThis,
    WalkKey,
    WalkOnward,
    /// A walk that hands out the items' own cells asks for this first: a
    /// thing that is its own walk has no such cells, and a language with
    /// words for that says so and stops.
    WalkAlone,
    /// Where a walk that keeps its place by the item it handed out goes
    /// on from. Below the cell of that item stand the place the pass was
    /// at and the array as it now stands, since the body may have moved
    /// the item, or taken it away altogether.
    WalkPast,
    /// Which of two values comes first: below, alike, or above.
    Rank,
    /// Whether two values are the very same: of one kind, and alike
    /// within it. 1 and 1.0 are equal but not the same.
    Same,
    Unsame,
    Contains,
    Lacks,
    /// The bits of two whole numbers taken together, and the bits of one
    /// turned over. A number is read as sixty-four bits, sign and all.
    BitBoth,
    BitEither,
    BitOne,
    BitTurn,
    /// The bits moved up or down that many places.
    BitUp,
    BitDown,
    /// The value of the binding whose name the text above spells: how a
    /// language reads a name worked out while the program runs.
    Named,
    /// The cell of the outermost binding a value names, made where
    /// that binding has none yet.
    BondNamed,
    /// Write the value above into the binding whose name the text under
    /// it spells.
    WriteNamed,
    /// Leave the binding a value spells standing for nothing.
    ForgetNamed,
    /// Make the binding a value spells hold nothing where it held
    /// nothing at all, so that reading it is reading a name written to.
    ReadyNamed,
    /// The cell of a place inside whatever the cell below the keys
    /// holds: how a place is shared out of something that is not a
    /// binding of its own, such as a property holding an array.
    BondWithin(usize),
    /// Make that own value of the class above a shared cell if it is
    /// not one already, and push the cell.
    BondOwn(Rc<str>),
    /// Take the place the key names out of whatever the cell below it
    /// holds, leaving everything else where it was.
    ForgetWithin,
    /// Fasten the binding a value spells to the cell above it, so the
    /// two names stand for the one cell.
    FastenNamed,
    /// The value above made a value of that kind (ext.op.cast).
    Cast(crate::value::Sort),
    /// Whether the value above is nothing at all.
    Nothing,
    /// Say these words about the piece standing here, in the language's
    /// own word for that kind of remark, and go on: how a definition
    /// that has something to say about a shape it still allows says it.
    Remark(crate::lang::Complaint, Rc<str>),
    /// The value standing here where a cell was asked for. Where it is
    /// a cell it is handed on as it is; where it is not, these words are
    /// said and it is handed on all the same, or, in the second, said
    /// and the run stopped there.
    HeldOrSaid(crate::lang::Complaint, Rc<str>),
    HeldOrStop(Rc<str>),
    /// A cell for the value standing here, whatever it is: the cell
    /// itself where it is one already, and otherwise these words and a
    /// fresh cell holding it. A routine written to give back a cell
    /// gives one back however it ends, and only the run can say whether
    /// what it named had one of its own.
    HeldAnyway(crate::lang::Complaint, Rc<str>),
    /// What an array holds at that place, answering nothing where it
    /// holds nothing there, or where what is asked is not an array at
    /// all, and saying nothing about it either way: how a language asks
    /// whether something is there without minding that it is not.
    Peek,
    /// What an array holds at that place, an empty array where it holds
    /// nothing there: how a write reaches a place within a place that
    /// is not there yet, and makes it on the way.
    Nested,
    /// Build the class this plan describes; what it stands on, if it
    /// stands on anything, is the value below.
    Forge(Rc<Plan>),
    /// A new object of the class below the arguments, its maker run.
    Make,
    /// The property of that name, of the object above.
    Grab(Rc<str>),
    /// Whether the value has this member, before choosing the pipe.
    HasMember(Rc<str>),
    /// Write that property: the object, then the value.
    Plant(Rc<str>),
    /// Take that property off the object above, as though it had never
    /// been written.
    Uproot(Rc<str>),
    /// Make that property of the object above a shared cell if it is
    /// not one already, and push the cell, so another name may be
    /// fastened to it.
    BondField(Rc<str>),
    /// The property of the object, named by the text above it: how a
    /// language reads a property whose name is worked out while the
    /// program runs.
    GrabNamed,
    /// Write that property: the object, the name, then the value.
    PlantNamed,
    /// Call the method of the object, named by the text above it, with
    /// the arguments above that. The count is of the arguments alone.
    SendNamed(usize),
    /// A class's own value, named by the text above the class.
    ReachNamed,
    /// Call the method of the class, named by the text above the
    /// arguments. The count is of the arguments alone.
    SummonNamed(usize),
    /// Write a class's own value: the class, the name, then the value.
    SowNamed,
    /// Call that method of the object below the arguments.
    Send(Rc<str>),
    /// A constant or a class's own value, of the class above.
    Reach(Rc<str>),
    /// Write a class's own value: the class, then the value.
    Sow(Rc<str>),
    /// Call that method of a named class: the object it is for, the
    /// class, then the arguments.
    Summon(Rc<str>),
    /// Whether the object above is of that class, or of one beneath it.
    Kindred(Rc<str>),
    /// Whether a thing is of the class a value stands for, where the
    /// class to test against is only known as the run reaches it.
    KindredTo,
    /// A routine written where a value stands, carrying away the values
    /// under it: the routine is on top, and each value below it fills
    /// one of the slots the routine names as carried.
    Close,
    /// The name of the class of the value above.
    Titled,
    /// Raise the value above as a fault to be caught.
    Hurl,
    Reraise,
    AssertFault,
    /// Whether the value above is of any of those classes; it is consumed.
    Matches(Rc<Vec<String>>),
    /// Push the value above a second time.
    Twin,
    /// Everything above the nearest mark as an array, the mark removed.
    Collect,
    Builtin(Builtin, Rc<str>),
}

/// Builtins a definition names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Repr, Hash, Bool, Sorted, Iter, Next, IsInstance,
    Sum,
    List,
    Any,
    Echo,
    Say,
    Out,
    /// Every argument's text, run together, no line end (ext.builtin.echo).
    Tell,
    /// A named global constant (ext.builtin.define); bound at compile time.
    Define,
    /// An array or map written as a call (ext.builtin.array).
    Pack,
    /// A value laid out over lines, PHP's print_r (ext.builtin.print_r).
    Layout,
    /// Say the run is over where it stands (ext.builtin.exit). Text is
    /// written out first; a number is not.
    Leave,
    /// Take a binding, or a place in an array, away (ext.builtin.unset).
    Erase,
    /// Put values at the head of a named array, the places after them
    /// moving along to make room (ext.builtin.array.front). What a place
    /// is called by a whole number is called anew from nought; what it
    /// is called by a word keeps that word. The answer is how many
    /// places the array holds afterwards.
    Lead,
    /// Whether each of those bindings, or places in an array, holds
    /// something other than nothing, asking without minding that a
    /// binding was never written or a place is not there
    /// (ext.builtin.isset).
    Held,
    /// Whether what a name or a place holds is untrue, asked the same
    /// way: without minding that it is not there at all
    /// (ext.builtin.empty). It is the companion of the one above, and
    /// asks the looser question, since nothing at all is untrue.
    Hollow,
    /// Each argument with its kind, PHP's var_dump (ext.builtin.var_dump).
    Dump,
    /// Source read while the program runs: the text itself
    /// (ext.builtin.eval), or the text a file holds
    /// (ext.builtin.include). It is assembled against the same globals
    /// and run where it stands, and what it gives back is its answer.
    Eval,
    Include,
    /// The same, but only where that file has not been read before in
    /// this run (ext.builtin.include.once); a file read already answers
    /// with truth and is not read again.
    IncludeOnce,
    /// What a file holds, all of it at once; what to write into one;
    /// whether a file is there at all; and taking one away
    /// (ext.builtin.file.*). Only a language that spells these reaches
    /// outside the run at all.
    FileRead,
    FileWrite,
    FileThere,
    FileGone,
    /// A command handed to the host's own shell, answering with all
    /// that the shell wrote where a run writes (ext.builtin.shell).
    /// Only a language that spells this may start another program at
    /// all.
    ShellSaid,
    NetAsk,
    Waited,
    RunBegin,
    RunEnd,
    /// How long the run may take from here, in seconds; nought lifts
    /// the limit (ext.builtin.time_limit).
    TimeLimit,
    /// The room the run has taken, in bytes: what it holds at this
    /// moment (ext.builtin.room.used), the most it ever held at once
    /// (ext.builtin.room.most), and the forgetting of that highest
    /// reading so that it is counted afresh from here
    /// (ext.builtin.room.most.forget).
    RoomUsed,
    RoomMost,
    RoomForget,
    /// How much room the run may take, in bytes; nought lifts the
    /// limit (ext.builtin.room.limit).
    RoomLimit,
    /// Keeping what the run writes out rather than letting it go
    /// (ext.builtin.output.*): begin keeping, what has been kept since
    /// the last beginning, stop keeping and give up what was kept, and
    /// how many keepings are in force. What a keeping gives up may be
    /// written out again by whoever asked for it, so a language builds
    /// flushing and filtering out of these four.
    HoldOut,
    HeldOut,
    DropOut,
    DeepOut,
    /// A routine to run when the run is over, with whatever else was
    /// given to stand as its arguments (ext.builtin.at_end). They run
    /// in the order they were named, after the program's own last
    /// statement and before what is still being kept is let go.
    WhenDone,
    /// A routine to be handed every complaint the run makes, instead of
    /// the complaint being written out (ext.builtin.complaint.handler).
    /// It is handed the word for the kind, what was said, where the
    /// program is written and which line was running. Answering false
    /// leaves the complaint to be written out as it would have been;
    /// giving nothing takes the routine away again.
    Complainer,
    /// Say these words as a complaint of the kind the first names, where
    /// the run stands (ext.builtin.complaint.say): how a language writes
    /// a complaint of its own and has it told as the run's own are, in
    /// its place and through whatever stands in their way.
    Complain,
    /// The names of the classes, and of the routines, the run has bound
    /// (ext.builtin.classes, ext.builtin.routines).
    ClassesBound,
    RoutinesBound,
    /// The words the language spells of its own, by name.
    Spelled,
    /// The methods a class answers to, and the properties its things
    /// hold, by name: what it has of its own and what it stands on has.
    ClassMethods,
    ClassProperties,
    /// How many seconds have passed since the start of the year the
    /// system counts from (ext.builtin.clock).
    Clock,
    /// The name of the class the one above stands on, where it stands on
    /// any: a thing is asked of its own class (ext.builtin.class.beneath).
    ClassBeneath,
    /// A working over the reals of the width, named by the first thing
    /// it is given and worked on the rest (ext.builtin.math): the roots,
    /// the curves and the angles a width of bits can be asked for but a
    /// definition has no words of its own for. One label for all of
    /// them, since it is the one power the kernel is lending.
    Math,
    /// Whether anything has gone out of the run yet: what is held back
    /// in a piece of output kept aside has not (ext.builtin.output.begun).
    OutBegun,
    /// A routine to be handed a value nobody took, rather than the run
    /// telling it in its own words (ext.builtin.uncaught). Giving
    /// nothing takes the routine away again.
    Untaken,
    /// The calls under way, innermost first, each an array telling the
    /// name called, the class it was written in where it was written in
    /// one, the file and line the call itself stands on, and what it was
    /// handed (ext.builtin.calls).
    Calls,
    /// What the running call was given, however much of it the routine
    /// named: all of it as an array, how much there was, or the one at a
    /// position (ext.builtin.args.*).
    Given,
    GivenCount,
    GivenAt,
    MakeReal,
    Places,
    ToText,
    ToInt,
    AsReal,
    Length,
    CharAtIndex,
    CodeOf,
    CharOf,
    Raise,
    SortOf,
    Numer,
    Denom,
    External,
    Append,
    Fetch,
    Replace,
    Span,
}

/// An operand a fused word reads without the stack: a binding or a constant.
#[derive(Debug, Clone)]
pub enum Operand {
    Cell(Cell),
    Const(Value),
    /// The top of the stack, popped.
    Top,
}

#[derive(Debug, Clone)]
pub enum Instr {
    /// Push a constant.
    Const(Value),
    /// Push a binding's value.
    Read(Cell),
    /// Pop into a binding.
    Write(Cell),
    /// Pop the arguments, apply the operation, push its result.
    Act(Action, usize),
    /// Pop; when the value is not true, continue at the index.
    Skip(usize),
    /// Whether the call left the frame's slot without a value.
    Missing(usize),
    /// Whether the global at this place has never been written.
    Unwritten(usize),
    /// Which line of the source the instrs after this one came from,
    /// so that a complaint can say where it happened.
    Line(u32),
    /// Start keeping complaints quiet, or stop: how a language that
    /// lets a program hush what one piece of it has to say about
    /// itself says where the quiet begins and ends.
    Hush(bool),
    /// Read this binding as it stands, saying nothing about it: where it
    /// holds nothing at all, nothing at all is what is read, so that
    /// writing it somewhere else leaves that place unwritten too.
    Glance(Cell),
    /// Make this binding a shared cell if it is not one already, and
    /// push that cell, so another name can be fastened to it.
    Bond(Cell),
    /// Pop a shared cell and put it in this binding, so the two names
    /// stand for one cell from here on.
    Fasten(Cell),
    /// Pop a place, make what the array in this binding holds there a
    /// shared cell, and push that cell: how a walk hands out its items
    /// for writing.
    BondItem(Cell),
    /// Pop that many keys, walk into the array in this binding by them,
    /// make what it holds at the last a shared cell, and push that cell.
    /// Unlike the one above this reads keys and not positions, and where
    /// a language makes what a write needs it makes the arrays and the
    /// places along the way.
    BondPlace(Cell, usize),
    /// Leave this binding as though nothing were ever written to it.
    Forget(Cell),
    /// Put nothing at all in this binding, letting go of whatever it
    /// held. The slot a routine keeps its running result in is emptied
    /// this way at the head of a statement, so that what the statement
    /// before came to is finished with before the next is worked out
    /// and not after it.
    Emptied(Cell),
    /// Pop the top of the stack and let it go. A statement written for
    /// what it does rather than for what it comes to ends with this, so
    /// that what it came to is finished with there and then and not held
    /// alive until something else takes its place.
    Shed,
    /// A word that does nothing at all. One stands where a word already
    /// written down has turned out not to be wanted and cannot be taken
    /// out without moving everything after it; the peephole takes them
    /// out at the end, when it is moving jumps in any case, so the
    /// engine never meets one.
    Nothing,
    /// From here until the mark that ends it, the run says nothing at
    /// all about itself: how a language lets a program silence a piece
    /// of itself outright, where Hush only keeps quiet about what is
    /// not there.
    Mute(bool),
    /// Make this global stand ready: where nothing was ever written to
    /// it, nothing is written to it now, so that a name bound to it is
    /// a name written to and not one never written.
    Ready(Cell),
    /// From here to the matching Unguard, a raised value is caught: the
    /// stack goes back to its depth here, the value is pushed, and the
    /// run goes on at the index.
    Guard(usize),
    Attempt(Box<Attempt>),
    Depart { to: usize, cycle: Option<usize> },
    Unguard,
    /// A binary operation whose operands come from bindings, constants or
    /// the stack, the result pushed: an operator that never touches the
    /// stack for a binding.
    Dyad { op: Action, a: Operand, b: Operand },
    /// A binding stepped by a constant in place.
    Bump { slot: Cell, by: Value },
    /// A comparison of two operands and a jump when it fails: the test of
    /// a loop or a branch in one word.
    SkipCmp { op: Action, a: Operand, b: Operand, to: usize },
}

/// A compiled program.
#[derive(Clone, Debug)]
pub struct Routine {
    pub ident: String,
    pub formals: Vec<String>,
    /// Ordinary, positional, named, gathered items, or gathered pairs.
    pub parameter_rules: Option<Vec<u8>>,
    /// The class each parameter is declared to take, where one was
    /// written and it names a class. Nothing for a parameter written
    /// without one, or with a kind that is not a class.
    pub formal_kinds: Vec<Option<Rc<str>>>,
    /// How many arguments must be given; the rest have a value of their
    /// own, written by the program's own first instrs.
    pub least: usize,
    pub rest_at: Option<usize>,
    /// Every local slot's name, the parameters first.
    pub idents: Vec<String>,
    /// A function leaves one value, its result; a postfix program leaves
    /// whatever it pushed.
    pub returns_value: bool,
    /// The program's own body, which nothing called: what a call was
    /// given cannot be read from within it.
    pub body_of_all: bool,
    /// The file this program was written in, where it came of text read
    /// while the run was going. A call of it is a call into that file:
    /// a complaint names it, and a file it asks for is looked for
    /// beside it. Nothing where the program is the run's own.
    pub written_in: Option<Rc<str>>,
    /// The class this program was written inside, where it was written
    /// inside one: what a class keeps to itself is reached from here and
    /// nowhere else.
    pub within: Option<Rc<str>>,
    /// The line the program was written on, which a fault raised on the
    /// way into it names: such a fault belongs where the program is
    /// written and not where the call stood.
    pub declared_on: u32,
    /// The slots a routine written where a value stands fills from what
    /// it carried away with it, in the order the names were written.
    pub carried: Vec<usize>,
    /// What one such routine carried away: a value for each of those
    /// slots. Empty for every routine written out under a name, which
    /// carries nothing.
    pub held: Vec<crate::value::Value>,
    pub instrs: Rc<Vec<Instr>>,
}

/// What a class declaration comes to: everything about the class that is
/// known while compiling. What it stands on is looked up when it runs.
#[derive(Debug)]
pub struct Plan {
    pub name: String,
    /// How many classes of method names only stand after the one this
    /// class is built on, when the class is forged.
    pub answers: usize,
    /// The names of the properties, of the values the class keeps for
    /// itself and of its constants; a value for each is on the stack, in
    /// that order, when the class is forged.
    pub field_names: Vec<String>,
    /// How far each property may be reached from, name for name.
    pub field_reach: Vec<crate::value::Reach>,
    pub shared_names: Vec<String>,
    pub constant_names: Vec<String>,
    pub methods: Vec<(String, Rc<Routine>)>,
    /// Whether a class to stand on is given first.
    pub extends: bool,
}
