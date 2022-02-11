// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

function sg_visualize(container, graph, paths) {
    let sg = container.append('svg').attr('class', 'sg');

    let nodes = sg.selectAll('.node')
        .data(graph.nodes, sg_node_to_id_str)
        .join('g')
        .attr('class', 'node')
        .each(sg_render_node);

    graph.edges.forEach((e) => { e.target = e.sink; });
    let edges = sg.selectAll('.edge')
        .data(graph.edges)
        .join('line')
        .attr('class', 'edge');

    let force = d3.forceSimulation(graph.nodes)
        .force("link", d3.forceLink(edges).id(sg_node_id_to_str))
        .force("charge", d3.forceManyBody())
        .force("x", d3.forceX())
        .force("y", d3.forceY())
        .on('tick', ticked)
        .stop();
    force.tick(100);
    ticked();

    function ticked() {
        nodes
            .attr('transform', (n) => `translate(${n.x}, ${n.y})`);
        edges
            .attr('x1', (e) => (e.source.x))
            .attr('y1', (e) => (e.source.y))
            .attr('x2', (e) => (e.target.x))
            .attr('y2', (e) => (e.target.y));
    }
}

function sg_render_node(node, idx, gs) {
    let g = d3.select(this);

    g.attr('class', `node ${node.type}`);

    switch (node.type) {
        case "drop_scopes":
            sg_text_box(g, "DROP");
            break;
        case "jump_to_scope":
            sg_text_box(g, "JUMP");
            break;
        case "pop_symbol":
            sg_text_box(g, node.symbol);
            break;
        case "pop_scoped_symbol":
            sg_text_box(g, node.symbol);
            break;
        case "push_symbol":
            sg_text_box(g, node.symbol);
            break;
        case "push_scoped_symbol":
            sg_text_box(g, node.symbol);
            break;
        case "root":
            sg_text_box(g, node.symbol);
            break;
        case "scope":
            g.append('circ');
            break;
    }
}

function sg_text_box(g, text) {
    text = g.append('text').text(text);
    console.log(text);
    bbox = text.node().getBBox();
    return g.insert('rect').lower()
        .attr('x', bbox.x)
        .attr('y', bbox.y)
        .attr('width', bbox.width)
        .attr('height', bbox.height);
}

function sg_node_to_id_str(node) {
    return sg_node_id_to_str(node.id);
}

function sg_node_id_to_str(id) {
    if (id.hasOwnProperty('file')) {
        return id.file + "#" + id.local_id;
    } else {
        return "#" + id.local_id;
    }
}