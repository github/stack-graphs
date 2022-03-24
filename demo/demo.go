package main

import (
	"context"
	"fmt"
	"io"
	"log"
	"os"
	"sort"
	"strings"

	sitter "github.com/smacker/go-tree-sitter"
	"github.com/smacker/go-tree-sitter/golang"
	"go.starlark.net/resolve"
	"go.starlark.net/starlark"
)

func main() {
	log.SetPrefix("")
	log.SetFlags(0)

	//const filename = "../../goproxy/cmd/goproxy/main.go"
	const filename = "./demo.go"

	// Parse the Go file.
	root, err := parse(golang.GetLanguage(), filename)
	if err != nil {
		log.Fatal(err)
	}

	// Parse and execute the Starlark script.
	resolve.AllowRecursion = true // TODO: make Starlark support bounded recursion depth (or use limit on steps?)
	thread := &starlark.Thread{}

	predeclared := starlark.StringDict{
		"node": starlark.NewBuiltin("node", makeNode),
		"edge": starlark.NewBuiltin("edge", makeEdge),
	}
	globals, err := starlark.ExecFile(thread, "./demo.star", nil, predeclared)
	if err != nil {
		handleEvalError(err)
	}

	// Print the entire syntax tree (debugging).
	if false {
		root.debug(os.Stderr, "root", 0)
	}

	// And call the main Starlark function on the root node.
	main := globals["main"]
	if main == nil {
		log.Fatalf("Starlark script has no main function")
	}
	if _, err := starlark.Call(thread, main, starlark.Tuple{root}, nil); err != nil {
		handleEvalError(err)
	}

	// Emit the Stack Graph nodes for each syntax node, plus their transitive closure.
	// TODO: Need a way to stick graph nodes in syntax nodes.
}

func handleEvalError(err error) {
	if evalErr, ok := err.(*starlark.EvalError); ok {
		log.Fatal(evalErr.Backtrace())
	}
	log.Fatal(err)
}

// Node is an immutable Starlark value that represents a Tree Sitter syntax node;
// see https://tree-sitter.docsforge.com/master/node/ for C API.
// We say "syntax" explicitly to avoid confusion with Stack Graph ("graph") nodes.
type syntaxNode struct {
	n    *sitter.Node
	file *file

	// TODO: record syntax/graph node association.
	//  syntaxnode.attr = graphnode? ambiguous with its properties.
	// Assume sitter.Nodes are canonical.
}

type file struct {
	name    string
	content []byte
}

// parse parses a file in the specified language and returns the root
// node of the Tree Sitter syntax tree.
func parse(lang *sitter.Language, filename string) (syntaxNode, error) {
	content, err := os.ReadFile(filename)
	if err != nil {
		return syntaxNode{}, fmt.Errorf("failed to read input file: %w", err)
	}

	root, err := sitter.ParseCtx(context.Background(), content, lang)
	if err != nil {
		return syntaxNode{}, err
	}

	file := &file{
		name:    filename,
		content: content,
	}
	return syntaxNode{file: file, n: root}, nil
}

var _ starlark.HasAttrs = syntaxNode{}

func (n syntaxNode) String() string      { return n.n.Type() }
func (syntaxNode) Type() string          { return "syntax-node" }
func (syntaxNode) Freeze()               {} // immutable
func (syntaxNode) Truth() starlark.Bool  { return true }
func (syntaxNode) Hash() (uint32, error) { return 0, nil } // TODO: implement

func (n syntaxNode) Attr(name string) (starlark.Value, error) {
	// core Tree Sitter node attributes
	switch name {
	case "__type__":
		return starlark.String(n.String()), nil

	case "__text__":
		return starlark.String(n.n.Content(n.file.content)), nil

	case "__location__":
		// TODO: define struct location { start, end position }.
		start, end := n.n.StartPoint(), n.n.EndPoint()
		loc := fmt.Sprintf("%s:%d:%d-%d:%d", n.file.name, start.Row+1, start.Column+1, end.Row+1, end.Column+1)
		return starlark.String(loc), nil

	case "__children__":
		elems := make([]starlark.Value, n.n.ChildCount())
		for i := range elems {
			elems[i] = syntaxNode{n: n.n.Child(i), file: n.file}
		}
		return starlark.NewList(elems), nil

	case "__debug__":
		var buf strings.Builder
		n.debug(&buf, "debug", 0)
		return starlark.String(buf.String()), nil
	}
	// Reserve the other double-underscore names (and reject misspellings).
	if strings.HasPrefix(name, "__") {
		return nil, nil
	}

	// fields defined by language grammar
	if child := n.n.ChildByFieldName(name); child != nil {
		return syntaxNode{n: child, file: n.file}, nil
	}

	// Because a Tree Sitter tree is schemaless, we must treat missing
	// fields (including misspellings) as None, not an error.
	return starlark.None, nil
}

func (n syntaxNode) AttrNames() []string {
	names := []string{"__type__", "__children__", "__text__", "__location__", "__debug__"}

	if false {
		// Broken, pending resolution of https://github.com/tree-sitter/tree-sitter/issues/1642.
		for i := 0; i < int(n.n.NamedChildCount()); i++ {
			name := n.n.FieldNameForChild(i)
			if name != "" && name != names[len(names)-1] { // de-dup
				names = append(names, name)
			}
		}
	} else {
		// workaround
		c := sitter.NewTreeCursor(n.n)
		for ok := c.GoToFirstChild(); ok; ok = c.GoToNextSibling() {
			if name := c.CurrentFieldName(); name != "" && name != names[len(names)-1] { // de-dup
				names = append(names, name)
			}
		}
		c.Close()
	}

	sort.Strings(names)
	return names
}

// debug writes to out the concrete syntax tree rooted at n.
func (n syntaxNode) debug(out io.Writer, name string, depth int) {

	if false {
		// broken due to https://github.com/tree-sitter/tree-sitter/issues/1642
		prefix := ""
		if name != "" {
			prefix = name + ": "
		}
		fmt.Fprintf(out, "%*s%s%v [%d-%d]\n", depth*4, "", prefix, n.n.Type(), n.n.StartByte(), n.n.EndByte())

		depth++
		for i := 0; i < int(n.n.ChildCount()); i++ {
			child := syntaxNode{file: n.file, n: n.n.Child(i)}
			child.debug(out, n.n.FieldNameForChild(i), depth)
		}
	} else {
		// workaround
		c := sitter.NewTreeCursor(n.n)
		defer c.Close()
		var visit func(depth int)
		visit = func(depth int) {
			prefix := ""
			if name := c.CurrentFieldName(); name != "" {
				prefix = name + ": "
			}
			n := c.CurrentNode()
			fmt.Fprintf(out, "%*s%s%s [%d-%d]\n", depth*4, "", prefix, n.Type(), n.StartByte(), n.EndByte())
			for ok := c.GoToFirstChild(); ok; ok = c.GoToNextSibling() {
				visit(depth + 1)
			}
		}
		visit(depth)
	}
}

// graph nodes
//
// n = node()              creates a new node.
// n.k = v                 sets the x attribute (which must not already exist) of the node to v.
// e = edge(n, m)          creates an edge n->m if it doesn't already exist, and returns it.
// e.k = v		   sets the x attribute (which must not already exist) of the edge to v.
//
// Q. is it appropriate for edge() to have get-or-create semantics but edge.k=v not to be idempotent?

func makeNode(thread *starlark.Thread, fn *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
	if len(args)+len(kwargs) > 0 {
		return nil, fmt.Errorf("node: unexpected arguments")
	}
	return new(graphNode), nil
}

func makeEdge(thread *starlark.Thread, fn *starlark.Builtin, args starlark.Tuple, kwargs []starlark.Tuple) (starlark.Value, error) {
	var from, to *graphNode
	if err := starlark.UnpackPositionalArgs("edge", args, kwargs, 2, &from, &to); err != nil {
		return nil, err
	}

	if from.edges == nil {
		from.edges = make(map[*graphNode]*graphEdge)
	}
	edge, ok := from.edges[to]
	if !ok {
		edge = new(graphEdge)
		from.edges[to] = edge
	}
	return edge, nil
}

type graphNode struct {
	attrs starlark.StringDict
	edges map[*graphNode]*graphEdge
}

var _ starlark.HasAttrs = (*graphNode)(nil)

func (n *graphNode) String() string { return "graph-node" }
func (n *graphNode) Type() string   { return "graph-node" }
func (n *graphNode) Freeze() {
	if n.attrs != nil {
		n.attrs.Freeze()
	}
}
func (n *graphNode) Truth() starlark.Bool  { return true }
func (n *graphNode) Hash() (uint32, error) { return 0, nil } // TODO: implement

func (n *graphNode) Attr(name string) (starlark.Value, error) { return n.attrs[name], nil }
func (n *graphNode) AttrNames() []string                      { return n.attrs.Keys() }
func (n *graphNode) SetField(name string, v starlark.Value) error {
	return setAttr(&n.attrs, "node", name, v)
}

type graphEdge struct {
	attrs starlark.StringDict
}

var _ starlark.HasAttrs = (*graphEdge)(nil)

func (e *graphEdge) String() string { return "graph-edge" }
func (e *graphEdge) Type() string   { return "graph-edge" }
func (e *graphEdge) Freeze() {
	if e.attrs != nil {
		e.attrs.Freeze()
	}
}
func (e *graphEdge) Truth() starlark.Bool  { return true }
func (e *graphEdge) Hash() (uint32, error) { return 0, fmt.Errorf("unhashable: graph-edge") }

func (e *graphEdge) Attr(name string) (starlark.Value, error) { return e.attrs[name], nil }
func (e *graphEdge) AttrNames() []string                      { return e.attrs.Keys() }
func (e *graphEdge) SetField(name string, v starlark.Value) error {
	return setAttr(&e.attrs, "edge", name, v)
}

func setAttr(attrs *starlark.StringDict, kind, name string, v starlark.Value) error {
	if *attrs == nil {
		*attrs = make(starlark.StringDict)
	}
	sz := len(*attrs)
	(*attrs)[name] = v
	if sz == len(*attrs) {
		return fmt.Errorf("%s already has .%s attr", kind, name)
	}
	return nil
}
