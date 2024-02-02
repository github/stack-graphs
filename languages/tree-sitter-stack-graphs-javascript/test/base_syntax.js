#!/usr/bin/node

// foo

export let x = 1;
import "foo";
debugger;
var x;
let x;
function foo() { }
function foo(a) { }
function foo(undefined) { }
function* foo() { }
class Foo { }
@Foo class Bar { }
{ }
if (true) { }
if (true) { } else { }
if (/**/ true) /**/ { } else /**/ { }
if (true) return;
if (true) return; else return;
if (/**/ true) /**/ return; else /**/ return;
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
return x;
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
function () { return; };
function () { return function () { }; };
() => { };
() => () => { };
function* () { yield 1; };
function (/**/) { };
function (x /**/) { };
function (/**/ x) { };
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
(x[i] = 1);
([x] = 1);
({ x: y } = 1);
({/**/x: y } = 1);
({x:y.z} = 1);
x += 1;
(1, 2);
1 ? 2 : 3;
class { };
<foo bar={baz}>
    <quux0.quux1.quux2 />
    <>doo</>
    {garply}
    { }
    {/**/x}
</foo>