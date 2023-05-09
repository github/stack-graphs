#!/usr/bin/node

// foo

export let x = 1;
import "foo";
debugger;
var x;
let x;
function foo() { }
function* foo() { }
class Foo { }
{ }
if (true) { }
switch (x) { }
for (x; y; z) { }
for (x in xs) { }
while (x) { }
do { } while (x);
try { throw x; } catch (e) { }
with (x) { }
break;
continue;
return;
;
foo: x;

(5);
"foo";
`foo`;
`f${"o"}o`
5;
x;
true;
false;
this;
super;
null;
undefined;
/foo/;
({});
{
    foo: "bar"
};
[];
[1, 2, 3];
function () { };
() => { };
function* () { };
foo();
foo(bar);
foo.bar;
foo[bar];
new Foo();
new Foo(bar);
await foo;
x++;
--x;
1 + 1;
-2;
(x = 1);
x += 1;
(1, 2);
1 ? 2 : 3;
yield 1;
class { };