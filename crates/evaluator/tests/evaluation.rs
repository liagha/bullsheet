use element::Value;
use evaluator::Source;

struct Sheet {
    lines: Vec<String>,
}

impl Sheet {
    fn of(text: &str) -> Self {
        let calculator = evaluator::Calculator::new();
        let readings = calculator.evaluate(&Source::from(text));
        Self {
            lines: readings
                .iter()
                .map(|reading| Self::render(reading.value()))
                .collect(),
        }
    }

    fn lines(&self) -> Vec<String> {
        self.lines.clone()
    }

    fn render(value: &Value) -> String {
        if let Some(number) = value.cast::<element::Number>() {
            return format!("number:{}", number.value());
        }
        if let Some(text) = value.cast::<element::Text>() {
            return format!("text:{}", text.as_ref());
        }
        if let Some(truth) = value.cast::<element::Truth>() {
            return format!("truth:{}", truth.value());
        }
        if let Some(key) = value.cast::<element::Key>() {
            return format!("key:{}", key);
        }
        if let Some(function) = value.cast::<element::Function>() {
            return format!("function:{}", function.name().as_ref());
        }
        if let Some(parameter) = value.cast::<element::Parameter>() {
            return format!("parameter:{}", parameter.name().as_ref());
        }
        if let Some(failure) = value.cast::<element::Failure>() {
            return format!("failure:{}", failure);
        }
        if value.cast::<element::Blank>().is_some() {
            return String::from("blank");
        }
        format!("other:{}", value.describe())
    }
}

const CASES: &[(&str, &[&str])] = &[
    ("2 + 3", &["number:5"]),
    ("2 + 3 * 4", &["number:14"]),
    ("(2 + 3) * 4", &["number:20"]),
    ("10 - 3 - 2", &["number:5"]),
    ("2 ** 10", &["number:1024"]),
    ("2 ^ 10", &["number:1024"]),
    ("2 ^ 3 ^ 2", &["number:512"]),
    ("5!", &["number:120"]),
    ("50%", &["number:0.5"]),
    ("2 (3 + 4)", &["number:14"]),
    ("1,234 + 1", &["number:1235"]),
    ("sin(90deg)", &["number:1"]),
    ("cos(0)", &["number:1"]),
    ("asin(0.5) / deg", &["number:30.000000000000004"]),
    ("pi", &["number:3.141592653589793"]),
    ("tau", &["number:6.283185307179586"]),
    ("e", &["number:2.718281828459045"]),
    ("phi", &["number:1.618033988749895"]),
    ("deg", &["number:0.017453292519943295"]),
    ("min(300, 100, 200)", &["number:100"]),
    ("max(1, 2, 3)", &["number:3"]),
    ("hypot(3, 4)", &["number:5"]),
    ("atan2(1, 1)", &["number:0.7853981633974483"]),
    ("log(8, 2)", &["number:3"]),
    ("log(100)", &["number:2"]),
    ("log2(8)", &["number:3"]),
    ("log10(1000)", &["number:3"]),
    ("round(3.14159, 2)", &["number:3.14"]),
    ("round(3.14159)", &["number:3"]),
    ("floor(1.9)", &["number:1"]),
    ("ceil(1.1)", &["number:2"]),
    ("trunc(1.9)", &["number:1"]),
    ("sign(-4)", &["number:-1"]),
    ("abs(-4)", &["number:4"]),
    ("exp(0)", &["number:1"]),
    ("sqrt(9)", &["number:3"]),
    ("cbrt(27)", &["number:3"]),
    ("1 == 1", &["truth:true"]),
    ("1 != 2", &["truth:true"]),
    ("2 < 1", &["truth:false"]),
    ("2 > 1", &["truth:true"]),
    ("2 <= 2", &["truth:true"]),
    ("2 >= 3", &["truth:false"]),
    ("true", &["truth:true"]),
    ("false", &["truth:false"]),
    ("not(true)", &["truth:false"]),
    ("and(true, false)", &["truth:false"]),
    ("or(true, false)", &["truth:true"]),
    ("if(1 == 1, 2, 1 / 0)", &["number:2"]),
    ("\"a\" + \"b\"", &["text:ab"]),
    ("Ctrl+Alt+C", &["key:Ctrl+Alt+C"]),
    ("Ctrl+1", &["key:Ctrl+1"]),
    ("Ctrl + 1", &["key:Ctrl+1"]),
    ("Ctrl+Alt+C+Shift", &["key:Ctrl+Alt+Shift+C"]),
    ("Space", &["key:Space"]),
    ("F8", &["key:F8"]),
    ("1 / 0", &["failure:Division by zero"]),
    ("1 % 0", &["failure:Remainder by zero"]),
    ("sqrt(-1)", &["failure:Sqrt requires a non-negative number"]),
    ("ln(0)", &["failure:Ln requires a positive number"]),
    ("asin(2)", &["failure:Asin requires a number between minus one and one"]),
    ("acos(2)", &["failure:Acos requires a number between minus one and one"]),
    ("factorial(-1)", &["failure:Factorial requires a non-negative integer"]),
    (
        "round(1, 20)",
        &["failure:Round requires a whole number of places between zero and fifteen"],
    ),
    ("sin(1, 2)", &["failure:Sin expects 1 argument"]),
    ("sin(\"x\")", &["failure:Expected a number, received a text"]),
    ("zzz", &["failure:Unknown name zzz"]),
    ("pi = 3", &["failure:Pi is reserved"]),
    ("1 +", &["failure:Unexpected end of expression"]),
    ("(1 + 2", &["failure:Expected ')'"]),
    ("", &["blank"]),
    ("# a comment", &["blank"]),
    ("x = 7\nx * 6", &["number:7", "number:42"]),
    ("double(n) = n * 2\ndouble(21)", &["function:double", "number:42"]),
    ("add(2, 3)", &["number:5"]),
    ("plus = +\nplus(3, 4)", &["function:add", "number:7"]),
    ("sine = sin\nsine(1)", &["function:sin", "number:0.8414709848078965"]),
    (
        "apply(g, a, b) = g(a, b)\napply(add, 2, 3)",
        &["function:apply", "number:5"],
    ),
    (
        "fact(n) = if(n == 0, 1, n * fact(n - 1))\nfact(5)",
        &["function:fact", "number:120"],
    ),
    ("2 + 3\nans * 2", &["number:5", "number:10"]),
    ("x = 1\ny = 2\nx + y", &["number:1", "number:2", "number:3"]),
    ("x = 7", &["number:7"]),
    ("x = 7\nx", &["number:7", "number:7"]),
    ("x = 7\nx = 9\nx", &["number:7", "number:9", "number:9"]),
    ("x = y = 1", &["number:1"]),
    ("x = 1\ny = 2\nx + y", &["number:1", "number:2", "number:3"]),
    ("x = 1 / 0", &["failure:Division by zero"]),
    ("x = 1 / 0\nx", &["failure:Division by zero", "failure:Unknown name x"]),
    ("(x = 1)", &["failure:Expected ')', found ="]),
    ("f(x = 1)", &["failure:Expected ',' or ')', found ="]),
    ("2 = 3", &["failure:A binding needs a name"]),
    ("x = 1 = 2", &["failure:A binding needs a name"]),
    ("f(1) = 2", &["failure:A parameter must be a name"]),
    ("f() = 2\nf()", &["function:f", "number:2"]),
    ("f(n, m) = n + m\nf(2, 3)", &["function:f", "number:5"]),
    ("f(n) = n * 2\nf(3)", &["function:f", "number:6"]),
    ("x = 1\nx = 2\nx", &["number:1", "number:2", "number:2"]),
    ("x = 1 + 2", &["number:3"]),
    ("1 + 2 = 3", &["failure:A binding needs a name"]),
    ("pi = 3", &["failure:Pi is reserved"]),
];

#[test]
fn expressions() {
    for (source, expected) in CASES {
        let expected: Vec<String> = expected.iter().map(|line| (*line).to_string()).collect();
        assert_eq!(Sheet::of(source).lines(), expected, "{source}");
    }
}

#[test]
fn names_are_known() {
    let calculator = evaluator::Calculator::new();
    let readings = calculator.evaluate(&Source::from(""));
    let names: Vec<String> = readings
        .names()
        .iter()
        .map(|name| name.text().as_ref().to_string())
        .collect();
    assert!(names.contains(&String::from("sin")));
    assert!(names.contains(&String::from("pi")));
    assert!(names.contains(&String::from("add")));
}
