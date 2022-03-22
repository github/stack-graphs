module demo

go 1.17

require github.com/smacker/go-tree-sitter v0.0.0-20220314031503-79c376d254d3

require (
	go.starlark.net v0.0.0-20220302181546-5411bad688d1 // indirect
	golang.org/x/sys v0.0.0-20200930185726-fdedc70b468f // indirect
)

replace github.com/smacker/go-tree-sitter => ../../../w/go-tree-sitter
