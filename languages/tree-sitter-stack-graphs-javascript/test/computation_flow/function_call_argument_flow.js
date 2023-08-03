let obj = {
    x: 1
};



// function declaration
function func_1(o) {
    return o;
}

func_1(obj).x;
//          ^ defined: 2



// generator function declaration
function* gen_func_1(o) {
    yield o;
}

gen_func_1(obj).x;
//              ^ defined: 2



// assigned function
let func_2 = function (o) {
    return o;
}

func_2(obj).x;
//          ^ defined: 2



// assigned generator function
let gen_func_2 = function* (o) {
    yield o;
}
gen_func_2(obj).x;
//              ^ defined: 2



// assigned single-param expression-body arrow function
let func_3 = o => o;

func_3(obj).x;
//          ^ defined: 2



// assigned multi-param expression-body arrow function
let func_4 = (o, p) => o;

func_4(obj, 1).x;
//             ^ defined: 2



// assigned single-param statement-body arrow function
let func_5 = o => { return o; };

func_5(obj).x;
//          ^ defined: 2



// assigned multi-param statement-body arrow function
let func_6 = (o, p) => { return o; };

func_6(obj, 1).x;
//             ^ defined: 2