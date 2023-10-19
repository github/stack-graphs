let x = { foo: 1 };

function f() {
   return arguments;
}

f(x)[0].foo;
//      ^ defined: 1

function* g() {
  return arguments;
}

g(x)[0].foo;
//      ^ defined: 1

let h = function () {
   return arguments;
};

h(x)[0].foo;
//      ^ defined: 1
