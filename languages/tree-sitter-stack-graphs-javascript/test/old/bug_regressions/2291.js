// need this to make the tests stop complaining about non-existing test cases
x = 1;
_ = x;
//  ^ defined: 2

f = function () { };
f = function* () { };
f = () => 1;

var f = function () { };
var f = function* () { };
var f = () => 1;

let f = function () { };
let f = function* () { };
let f = () => 1;

f.p = function () { };
f.p = function* () { };
f.p = () => 1;

// related assignment-like things:

{
    f: function () { },
    f: function* () { },
    f: () => 1
};