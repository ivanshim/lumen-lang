// The seven forms and what they name.
//
// A constant, a binding read, a binding written, a call: the floor of
// the fourth design's ancestor. A cycle, a dyad and a bump: the three
// the kernel lab found worth a form, each removing a routine call, an
// argument list or a form visit from a hot path.

use std::rc::Rc;

/// What is put after a class's name to file it under: a class and a
/// routine may go by one name, and a binding is one thing again, so the
/// three are told apart by how each is filed and not by what it holds.
pub const OF_A_CLASS: &str = "\0class";

use crate::data::Value;

/// Where a binding lives: `depth` frames up, at `index`. A name that has
/// no binding there yet is read from the global slot instead.
#[derive(Debug, Clone)]
pub struct Address {
    pub ident: Rc<str>,
    pub up: usize,
    pub at: usize,
    pub fallback: Option<usize>,
}

/// What a call reaches: a kernel operation, or a program value.
#[derive(Debug)]
pub enum Callee {
    Prim(Prim, Rc<str>),
    Code(Box<Form>),
}

/// The kernel's operations. A definition maps surface names onto the
/// first group; the second group carries the control the tree lacks
/// forms for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prim {
    /// A step onward or back (`++`, `--`): adding or taking away one,
    /// save where a language walks text along its letters instead.
    Onward(bool),
    // builtins a definition may spell
    Echo,
    Say,
    Out,
    /// Each argument's text in turn, nothing between, no line end.
    Tell,
    /// A named global constant, bound when built (ext.builtin.define).
    Define,
    /// An array or a map written like a call (ext.builtin.array).
    Gather,
    /// A value written over lines, PHP's print_r (ext.builtin.print_r).
    Portray,
    /// Which of two values comes first: below, alike or above.
    Rank,
    /// The first value when it is something, else the second, which is
    /// worked out only then.
    Otherwise,
    /// Give an array back without the place named (ext.builtin.unset).
    Erase,
    /// What an array holds at that place, answering nothing where it
    /// holds nothing there and where what is asked is not an array,
    /// with nothing said about it: how a language asks whether
    /// something is there without minding that it is not.
    Glance,
    /// Whether every one of these is something other than nothing.
    Standing,
    /// Whether what a name or a place holds is untrue, asked as gently:
    /// without minding that it is not there at all (ext.builtin.empty).
    /// The companion of the one above, asking the looser question, since
    /// nothing at all is untrue.
    Hollow,
    /// The value made one of the whole kind (ext.op.cast).
    AsWhole,
    /// The value made one of the real kind.
    AsDecimal,
    /// The value made text.
    AsChars,
    /// The value made a flag.
    AsTruth,
    /// The value made an array; anything that is not one becomes an
    /// array holding just itself.
    AsVector,
    /// The value made nothing at all.
    AsNothing,
    /// Say the run is over where it stands (ext.builtin.exit). Text
    /// given is written out first; a number is not.
    Quit,
    /// What an array holds at that place, an empty array where it holds
    /// nothing there: how a write reaches into a place that is not there
    /// yet, making it on the way in.
    Inward,
    /// Source read while the program runs: the text itself
    /// (ext.builtin.eval), or the text a file holds
    /// (ext.builtin.include). It is built against the globals the run
    /// already has and run where it stands, and what it answers with is
    /// what it gives back.
    Weigh,
    Bring,
    /// The same as the one above, but only where that file has not been
    /// read before in this run (ext.builtin.include.once); one read
    /// already answers with truth and is not read again.
    BringOnce,
    /// What a file holds, all at once; what to put into one; whether a
    /// file is there; and taking one away (ext.builtin.file.*). A
    /// language reaches outside its run only by spelling these.
    Slurp,
    Spill,
    There,
    Gone,
    /// How long the run may take from here, counted in seconds; nought
    /// takes the limit away (ext.builtin.time_limit).
    Clock,
    /// Keeping what the run writes out instead of letting it go
    /// (ext.builtin.output.*): begin keeping, what has been kept since
    /// the last beginning, stop keeping and give up what was kept, and
    /// how many keepings are in force. What a keeping gives up may be
    /// written out again by whoever asked for it, so flushing and
    /// filtering are built of these four rather than spelled apart.
    KeepOut,
    KeptOut,
    LooseOut,
    DeepOut,
    /// A routine to run once the run is over, with whatever else was
    /// given standing as its arguments (ext.builtin.at_end). They run in
    /// the order they were named, after the program's last statement and
    /// before what is still kept is let go.
    Afterward,
    /// A routine to be handed every complaint the run makes, instead of
    /// the complaint being written out (ext.builtin.complaint.handler).
    /// It takes the word for the kind, what was said, where the program
    /// is written and the line that was running. Answering false leaves
    /// the complaint to be written out as it would have been; giving
    /// nothing takes the routine away again.
    Hearer,
    /// Say these words as a complaint of the kind the first names, where
    /// the run stands (ext.builtin.complaint.say): how a language makes
    /// a complaint of its own and has it told as the run's own are, in
    /// its place and through whatever stands in their way.
    Complain,
    /// The names of the classes, and of the routines, the run has bound
    /// (ext.builtin.classes, ext.builtin.routines).
    ClassesBound,
    RoutinesBound,
    /// The words the language has of its own, by name.
    WordsSpelled,
    /// How many seconds have passed since the start of the year the
    /// system counts from (ext.builtin.clock).
    SinceEpoch,
    /// The name of the class the one above stands on, where it stands on
    /// any: a thing is asked of its own class (ext.builtin.class.beneath).
    ClassBeneath,
    /// Whether anything has gone out of the run yet: what is held back
    /// in a piece of output kept aside has not (ext.builtin.output.begun).
    OutBegun,
    /// A routine to be handed a value nobody took, rather than the run
    /// telling it in its own words (ext.builtin.uncaught). Giving
    /// nothing takes the routine away again.
    Untaken,
    /// The calls under way, innermost first, each a table telling what
    /// was called, the class it was written in where it was written in
    /// one, the file and line the call itself stands on, and what it was
    /// handed (ext.builtin.calls).
    Under,
    /// What the running call was handed, whatever of it the routine
    /// gave names to: the whole of it, how much there was, or the one
    /// standing at a place (ext.builtin.args.*).
    Handed,
    HowMany,
    HandedAt,
    /// Whether two values are one and the same, which asks more than
    /// being equal: they must also be of one kind, so 1 and 1.0 are
    /// equal without being the same.
    Selfsame,
    Unlike,
    /// The bits of a value, sixty-four of them, sign and all: both set,
    /// either set, one alone set, all turned over, and moved up or down.
    /// Two pieces of text take their bits letter by letter instead.
    BitsBoth,
    BitsEither,
    BitsOne,
    BitsOver,
    BitsUp,
    BitsDown,
    /// Each argument with its kind, as PHP's var_dump (ext.builtin.var_dump).
    Dump,
    MakeReal,
    Places,
    AsText,
    AsInt,
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
    /// A map from the values given: a couple is a key and its value,
    /// anything else takes the next whole number as its key.
    MakeMap,
    /// `k => v` held as one value until a literal takes it in.
    Couple,
    /// The key, or the item, at a position: how a foreach walks.
    KeyAt,
    ItemAt,
    /// How many places an array or a map holds.
    Extent,
    /// Whether a thing still keeps a member at the place a walk has
    /// reached. Where it does not, the pass is passed over. The thing
    /// comes first, the place after it.
    Kept,
    /// What a walk walks. A thing handing another over to be walked in
    /// its stead answers with that one, and so on until one does not; a
    /// thing that is its own walk is wound back and answers with itself;
    /// anything else answers with itself.
    Walked,
    /// Whether a walk has more to hand out, what is at hand, what it is
    /// named, and the step onward. A thing that is its own walk is
    /// asked; anything else is counted through, as an array is.
    MoreYet,
    AtHand,
    NamedHere,
    StepOn,
    /// Asked before a walk that hands out the items' own cells: a thing
    /// that is its own walk keeps no such cells, and a language with
    /// words for that says so and stops.
    AloneWalk,
    /// `a[]`, a place only a store reaches.
    AtEnd,
    /// A thing of the class given, its maker run over the rest.
    Spawn,
    /// The property named by the second value, of the thing in the first.
    Of,
    /// Write that property: the thing, the name, the value.
    Onto,
    /// Take that property off the thing, as though it had never been
    /// written there.
    Pluck,
    /// Call the method named second, of the thing named first.
    Ask,
    /// A constant or a kept value of the class given.
    Within,
    /// Write a kept value: the class, the name, the value.
    Into,
    /// Call a method of a named class: what it is for, the class, the
    /// name, then the arguments.
    Bid,
    /// Whether the first is a thing of the class named second.
    Akin,
    /// The name of the class of a thing, or of a class.
    Named,
    /// Raise the value given, for a clause to take.
    Hurl,
    /// An array with one more item, or with one place written: the
    /// array itself is left alone.
    Added,
    Placed,
    // operators
    Plus,
    Minus,
    Times,
    Over,
    OverReal,
    IntDiv,
    Mod,
    Power,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Invert,
    Negate,
    Join,
    At,
    /// What a value holds at that place, read while the value is being
    /// taken apart: the same as the above, save that a value with no
    /// places at all is spoken of in a taking-apart's own words.
    Apart,
    /// What a value holds at that place, read so that what comes of it
    /// may be written back there. A value with no places at all is no
    /// place to write, so it is turned down as a write to one is.
    Toward,
    MakeArray,
    // control
    Seq,
    Choose,
    Both,
    Either,
    Yield,
    Leave,
    Resume,
}

#[derive(Debug)]
pub enum Form {
    Const(Value),
    Read(Address),
    Write(Address, Box<Form>),
    Apply(Callee, Vec<Form>),
    /// A loop as a form, run in the frame it appears in, instead
    /// of a program that calls itself. `after` tests after the body.
    Cycle { test: Box<Form>, body: Box<Form>, step: Option<Box<Form>>, after: bool },
    /// An operation of two operands, evaluated without a vector
    /// of arguments.
    Dyad { op: Prim, name: Rc<str>, a: Input, b: Input },
    /// `x = x + k`, the binding stepped in place.
    Bump { slot: Address, by: i64 },
    /// A class declaration: what is known while building, and a value
    /// for each property, kept value and constant the plan names.
    Class { plan: Rc<Plan>, values: Vec<Form> },
    /// A body run with clauses ready to take what it raises, and a last
    /// part that runs however the body ends.
    Attempt { body: Box<Form>, clauses: Vec<Clause>, last: Option<Box<Form>> },
    /// Whether the call left this binding without a value.
    Missing(Address),
    /// A statement together with the line of the source it was written
    /// on, so that a complaint can say where it happened.
    OnLine(u32, Box<Form>),
    /// The binding's own cell, made shareable if it is not already, so
    /// another name can be tied to it.
    Share(Address),
    /// Tie a name to a shared cell, past whatever it held before.
    Tie(Address, Box<Form>),
    /// Make what the array in this binding holds at that place a shared
    /// cell, and give it back: how a walk hands out its items.
    ShareItem(Address, Box<Form>),
    /// The same, but by the keys places are named by rather than by
    /// position, and down a chain of them; where a language makes what a
    /// write needs, the arrays and the places are made on the way in.
    SharePlace(Address, Vec<Form>),
    /// Make that property of the thing a shared cell if it is not one
    /// already, and give the cell back, so a name may be tied to it.
    ShareField(Box<Form>, Rc<str>),
    /// Make that value of the class a shared cell if it is not one
    /// already, and give the cell back.
    ShareOwn(Box<Form>, Rc<str>),
    /// Make the outermost binding this value spells a shared cell if it
    /// is not one already, and give the cell back.
    ShareCalled(Box<Form>),
    /// The cell of a place inside whatever a cell already holds: how a
    /// place is shared out of something that is no binding of its own,
    /// such as a property or a class's own value holding an array.
    ShareWithin(Box<Form>, Vec<Form>),
    /// Take the place the key names out of what a cell holds, leaving
    /// the rest of it where it was.
    ForgetWithin(Box<Form>, Box<Form>),
    /// Leave the outermost binding this value spells as though nothing
    /// were ever written to it.
    ForgetCalled(Box<Form>),
    /// Make the outermost binding this value spells ready, as the form
    /// above does for one written out.
    ReadyCalled(Box<Form>),
    /// Tie the outermost binding a value spells to a shared cell, so
    /// that name and the cell's other names stand for the one cell.
    TieCalled(Box<Form>, Box<Form>),
    /// Leave this binding as though nothing were ever written to it.
    Forget(Address),
    /// Make this global ready: where nothing was ever written to it,
    /// nothing is written to it now, so that a name bound to it names
    /// a binding written to and not one never written.
    Ready(Address),
    /// Find this value with whatever it has to say about itself kept
    /// quiet: how a language that lets a program silence one piece of
    /// itself says which piece.
    Muted(Box<Form>),
    /// The same, but the run says nothing at all of the piece within,
    /// not even a word about how it is written: how a language lets a
    /// program silence a piece of itself outright, where Muted only
    /// keeps quiet about what is not there.
    Silenced(Box<Form>),
    /// The binding whose name this value spells, found while the run
    /// goes: how a language reads a name it works out.
    Called(Box<Form>),
    /// Write the second into the binding whose name the first spells.
    CallWrite(Box<Form>, Box<Form>),
    /// Words a definition holds ready for a shape it still allows, said
    /// where the shape is reached and nowhere else, under the language's
    /// own word for that kind of remark. The line is the one the shape
    /// was written on, so that a call along the way does not move it.
    Remark(&'static str, Rc<str>, u32),
    /// The value the form within comes to, where a cell was asked for.
    /// Where it is a cell it goes on as it is; where it is not, these
    /// words are said on the line given and it goes on all the same, or,
    /// where nothing is said, the run is stopped with them.
    CellOrSaid(Option<&'static str>, Rc<str>, u32, Box<Form>),
    /// A cell for what the form within comes to, whatever that is: the
    /// cell itself where it is one already, and otherwise these words,
    /// on the line given, and a fresh cell holding the value. A routine
    /// written to give back a cell gives one however it ends, and only
    /// the run can say whether what it named had one of its own.
    HeldEither(&'static str, Rc<str>, u32, Box<Form>),
}

/// One catch: the classes it takes, where it holds what it caught, and
/// what it does with it.
#[derive(Debug)]
pub struct Clause {
    pub classes: Vec<String>,
    pub held: Option<Address>,
    pub body: Form,
}

/// A class as the builder knows it. What it is built on, and the value
/// of every member, are worked out when the declaration runs.
#[derive(Debug)]
pub struct Plan {
    pub name: String,
    /// How many classes of method names only follow the one this class
    /// is built on, among the values written for it.
    pub answers: usize,
    pub field_names: Vec<String>,
    /// How far each property is reached from, name for name.
    pub field_reach: Vec<crate::data::Reach>,
    pub shared_names: Vec<String>,
    pub constant_names: Vec<String>,
    pub methods: Vec<(String, Rc<Routine>)>,
    pub extends: bool,
}

/// An operand of a dyad that is a binding or a
/// constant is read directly, without a visit to a node.
#[derive(Debug)]
pub enum Input {
    Form(Box<Form>),
    Address(Address),
    Const(Value),
}

impl Input {
    pub fn of(node: Form) -> Input {
        match node {
            Form::Read(slot) => Input::Address(slot),
            Form::Const(v) if !matches!(v, Value::Routine(_)) => Input::Const(v),
            other => Input::Form(Box::new(other)),
        }
    }
}

/// What a program stops when it is raised inside: a function stops a
/// return, the program a loop is made of stops a break, a loop's body
/// stops a continue, and a branch arm stops nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Traps {
    Naught,
    Yields,
    Leaves,
    Resumes,
}

#[derive(Debug)]
pub struct Routine {
    /// How many arguments must be given; the rest carry a value of their
    /// own, written by the body's first forms.
    pub least: usize,
    pub ident: String,
    pub formals: Vec<String>,
    /// The class each parameter is written to take, where one was
    /// written and it names a class. Nothing for a parameter with none,
    /// or with a kind that is no class.
    pub formal_kinds: Vec<Option<Rc<str>>>,
    pub formal_slots: Vec<usize>,
    pub idents: Vec<String>,
    /// Holds no idents: runs in the frame it closed over, making none.
    pub frameless: bool,
    pub traps: Traps,
    /// The file this program was written in, where it came of text read
    /// as the run went. A call of it is a call into that file: a
    /// complaint names it, and a file it asks for is sought beside it.
    /// Nothing where the program is the run's own.
    pub written_in: Option<Rc<str>>,
    /// The class this program was written inside, where it was written
    /// inside one: what a class holds alone is reached from there and
    /// from nowhere else.
    pub within: Option<Rc<str>>,
    /// The line this program was written on, which a fault raised on
    /// the way into it names: such a fault belongs where the program
    /// stands and not where the call did.
    pub declared_on: u32,
    pub body: Form,
}
