function bark() {
    console.log("Bark!");
}

function meow() {
    console.log("Meow!");
}

function speak() {
    bark();
//  ^ defined: 1
    meow();
//  ^ defined: 5
}