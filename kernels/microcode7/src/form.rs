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
#[derive(Debug, Clone)]
pub enum Callee {
    Prim(Prim, Rc<str>),
    Code(Box<Form>),
}

/// The kernel's operations. A definition maps surface names onto the
/// first group; the second group carries the control the tree lacks
/// forms for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prim {
    /// A compound write may ask a real to retain its point.
    Pointed,
    Adorn(char),
    StartContext,
    DistinctObjects,
    Repr,
    Iterate,
    NextOne,
    FormatValue,
    MakeHeir,
    CallResult,
    CopyWorth,
    LoadModule,
    IsInstance,
    WriteMember,
    ReadMember,
    ProgramNames,
    BringModule,
    SpreadModule,
    /// Whether a member, rather than the pipe, takes the name.
    HasMember,
    /// Read a matrix product; the run cannot yet ask its methods.
    MatrixProduct,
    Membership,
    Dictionary,
    Suspend,
    Delegate,
    Following,
    Iterator,
    Tupled,
    MakeTuple,
    ValueMethod,
    BindValueMethod,
    SortedValues,
    Belongs, Tupling, Uniques, Ordered, Backwards, Numbered, Zipped, Mapped, Filtered, EveryTrue, Least, Greatest, Magnitude, Rounded, QuotRem, Powered, Hexadecimal, Octal, Binary, Quoted, Truthful, CallableValue, IdentityOf, Hashed, NextItem, HasAttribute, GetMember, SetMember, DropMember, MembersOf,
    SetCall(u8),
    /// Gather the parts naming a span within brackets.
    SliceBounds,
    /// A slice form kept readable while its running remains wanting.
    SliceRefused,
    Total,
    Listed,
    SomeTrue,
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
    /// The value, specification and conversion of a field in text.
    RenderField,
    /// Stop upon reaching a character the run cannot represent.
    UnheldText,
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
    /// Everything the host's own shell wrote out, having been handed a
    /// command to run (ext.builtin.shell). Starting a second program
    /// beside this one is something only a language spelling this may
    /// ask for.
    Shelled,
    Reached,
    Bided,
    Raised,
    Laid,
    /// How long the run may take from here, counted in seconds; nought
    /// takes the limit away (ext.builtin.time_limit).
    Clock,
    /// What the run has taken of the room the host hands out, counted in
    /// bytes: what it holds as things stand (ext.builtin.room.used), the
    /// highest it ever stood at (ext.builtin.room.most), and that highest
    /// reading thrown away so that it is gathered again from this moment
    /// (ext.builtin.room.most.forget).
    RoomHeld,
    RoomHighest,
    RoomAnew,
    /// How much room the run may take from here, counted in bytes;
    /// nought takes the mark away (ext.builtin.room.limit).
    RoomMark,
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
    /// The methods a class answers to, and the properties its things
    /// hold, by name: its own and those of the class it is built on.
    ClassMethods,
    ClassProperties,
    /// A routine written where a value stands, taking away with it the
    /// values after it: the routine comes first, and each value after
    /// fills one of the slots the routine names as carried.
    Carry,
    /// How many seconds have passed since the start of the year the
    /// system counts from (ext.builtin.clock).
    SinceEpoch,
    /// The name of the class the one above stands on, where it stands on
    /// any: a thing is asked of its own class (ext.builtin.class.beneath).
    ClassBeneath,
    /// A working over the reals of the width, named by the first worth
    /// handed over and worked on the rest (ext.builtin.math): the roots,
    /// the curves and the angles a width of bits can be asked for and a
    /// definition has no words of its own for. One label covers them
    /// all, since the one power lent is the working at the width.
    Reckon,
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
    Contains,
    Absent,
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
    /// The place a walk that keeps its place by the item it handed out
    /// takes up again: what is walked, the place the pass stood at, and
    /// the cell of the item handed out there, which the body may have
    /// carried elsewhere in the array or taken out of it.
    PastHeld,
    /// Put values before everything a named array holds, the places
    /// after them moving along (ext.builtin.array.front). A place named
    /// by a whole number is named anew from nought; one named by a word
    /// keeps its word. The answer is how many places there are then.
    Front,
    /// `a[]`, a place only a store reaches.
    AtEnd,
    /// The first value made what a place in the second will hold. Where
    /// the second is text and the language writes into text, a place
    /// there holds one letter, so only the first letter of the value
    /// goes in, and the language says as much where more than one was
    /// handed over; anything else is answered with as it stands. It is
    /// what tells a write into text what the write itself is worth.
    Letter,
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
    Positive,
    NumberAlone,
    Join,
    At,
    /// What a value holds at that place, read while the value is being
    /// taken apart: the same as the above, save that a value with no
    /// places at all is spoken of in a taking-apart's own words.
    Apart,
    /// Make one value for each target, gathering the starred middle.
    Partition(usize, Option<usize>),
    /// Gather consecutive portions of one comma expression.
    TupleJoined,
    /// What a value holds at that place, read so that what comes of it
    /// may be written back there. A value with no places at all is no
    /// place to write, so it is turned down as a write to one is.
    Toward,
    MakeArray,
    EmptySet,
    SetAssign(u8),
    /// The growing literal and the next part of it.
    ExtendLiteral(bool, bool),
    Iterated,
    CheckUnpack(usize),
    BindingWidth(usize),
    // control
    Seq,
    Choose,
    Both,
    Either,
    Yield,
    Leave,
    Resume,
}

/// The shape a case asks for; names are kept apart from their addresses.
#[derive(Debug, Clone)]
pub enum CaseTest {
    Ignore,
    Keep(String),
    Equal(Value),
    AnyOf(Vec<CaseTest>),
    Series { members: Vec<CaseTest>, spread: Option<usize> },
    Also { test: Box<CaseTest>, name: String },
    Pending(Vec<CaseTest>),
}

impl CaseTest {
    pub fn names(&self) -> Result<std::collections::BTreeSet<String>, ()> {
        use std::collections::BTreeSet;
        let mut found = BTreeSet::new();
        let children = match self {
            Self::Ignore | Self::Equal(_) => return Ok(found),
            Self::Keep(n) => { found.insert(n.clone()); return Ok(found); }
            Self::Also { test, name } => {
                found = test.names()?;
                if !found.insert(name.clone()) { return Err(()); }
                return Ok(found);
            }
            Self::AnyOf(arms) => {
                for (i, arm) in arms.iter().enumerate() {
                    let names = arm.names()?;
                    if i != 0 && found != names { return Err(()); }
                    found = names;
                }
                return Ok(found);
            }
            Self::Series { members, .. } | Self::Pending(members) => members,
        };
        for child in children {
            for name in child.names()? {
                if !found.insert(name) { return Err(()); }
            }
        }
        Ok(found)
    }
}

#[derive(Debug, Clone)]
pub enum Form {
    /// A case test which writes its names only upon success. A tuple
    /// may give up its members, but cannot yet be kept whole.
    Fits { value: Box<Form>, test: Rc<CaseTest>, slots: Vec<(String, Address)>, tuple: bool },
    Const(Value),
    Read(Address),
    /// The same, read as it stands and with nothing said about it: a
    /// binding that holds nothing at all reads as nothing at all, so
    /// that writing it elsewhere leaves that place unwritten too.
    Glance(Address),
    Write(Address, Box<Form>),
    Apply(Callee, Vec<Form>),
    /// A loop as a form, run in the frame it appears in, instead
    /// of a program that calls itself. `after` tests after the body.
    Cycle { test: Box<Form>, body: Box<Form>, step: Option<Box<Form>>, after: bool, otherwise: Option<Box<Form>> },
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
    Again,
    Assert { condition: Box<Form>, message: Box<Form> },
    Attempt { context: Option<Address>, body: Box<Form>, clauses: Vec<Clause>, last: Option<Box<Form>>, otherwise: Option<Box<Form>> },
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
#[derive(Debug, Clone)]
pub struct Clause {
    pub classes: Vec<String>,
    pub choices: Option<Vec<Form>>,
    pub grouped: bool,
    pub takes_all: bool,
    pub held: Option<Address>,
    pub body: Form,
}

/// A class as the builder knows it. What it is built on, and the value
/// of every member, are worked out when the declaration runs.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Routine {
    /// Method parameters whose fallback is evaluated in the body.
    pub local_defaults: Vec<usize>,
    pub generator: bool,
    /// How many arguments must be given; the rest carry a value of their
    /// own, written by the body's first forms.
    pub least: usize,
    pub gather_from: Option<usize>,
    pub ident: String,
    pub formals: Vec<String>,
    /// How each place is filled: both ways, by position, by name, or gathered.
    pub taking: Option<Vec<char>>,
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
    /// The slots a routine written where a value stands fills from what
    /// it carried away with it, in the order the names were written.
    /// Empty for every routine written out under a name.
    pub carried: Vec<usize>,
    pub body: Form,
}
