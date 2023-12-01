// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

'use strict';

class StackGraph {

    static margin = 6;
    static distx = 15;
    static disty = 15;

    static arrow_stem_w = 2;
    static arrow_head_w = 16;
    static arrow_head_h = 8;

    static number_of_file_colors = 8;

    constructor(container, graph, paths, metadata) {
        this.metadata = metadata;

        this.graph = graph;
        this.paths = paths;
        this.compute_data();

        this.current_node = null;
        this.current_edge = null;
        this.current_orient = { y: "south", x: "east" };
        this.paths_lock = null;
        this.render();
    }

    compute_data() {
        this.F = {};
        this.ID = {};
        this.N = [];
        this.compute_file_data();
        this.compute_node_data();
        this.compute_path_data();
    }

    compute_file_data() {
        for (let i in graph.files) {
            const file = graph.files[i];
            this.F[file] = i;
        }
        console.log(this.F);
    }

    compute_node_data() {
        for (let i in graph.nodes) {
            const node = graph.nodes[i];
            node.paths = []
            this.ID[this.node_to_id_str(node)] = i;
            this.N.push(node);
        }
    }

    compute_path_data() {
        const jumps = {};
        for (let path of this.paths) {
            const node_ids = path.edges.map((e) => e.source);
            node_ids.push(path.end_node);
            const nodes = {};
            const edges = {};
            nodes[this.node_id_to_str(node_ids[0])] = {
                stacks: [],
            };
            for (let i = 1; i < node_ids.length; i++) {
                const source = node_ids[i - 1];
                const sink = node_ids[i];
                const edge_id = this.edge_to_id_str({ source, sink });
                edges[edge_id] = {};
                nodes[this.node_id_to_str(sink)] = {
                    stacks: [],
                };
                // create jump edges, which are not part of the graph
                if (this.N[this.ID[this.node_id_to_str(source)]].type === 'jump_to_scope' && jumps[edge_id] !== true) {
                    jumps[edge_id] = true;
                    this.graph.edges.push({
                        precedence: 0,
                        source,
                        sink,
                        is_jump: true,
                    });
                }
            }
            path.derived = { nodes, edges };
            this.compute_path_stacks(path);
            this.N[this.ID[this.node_id_to_str(path.start_node)]].paths.push(path);
        }
    }

    compute_path_stacks(path) {
        let symbol_stack = null;
        let scope_stack = null;
        var index = 0;
        for (; index < path.edges.length; index++) {
            const edge = path.edges[index];
            const node_id = this.node_id_to_str(edge.source);
            const node = this.N[this.ID[node_id]];
            [symbol_stack, scope_stack] = this.compute_stacks_after_node(node, symbol_stack, scope_stack);
            path.derived.nodes[node_id].stacks.push({
                index,
                symbol_stack,
                scope_stack,
            });
        }
        const node_id = this.node_id_to_str(path.end_node);
        const node = this.N[this.ID[node_id]];
        [symbol_stack, scope_stack] = this.compute_stacks_after_node(node, symbol_stack, scope_stack);
        path.derived.nodes[node_id].stacks.push({
            index,
            symbol_stack,
            scope_stack,
        });
    }

    compute_stacks_after_node(node, symbol_stack, scope_stack) {
        switch (node.type) {
            case "drop_scopes":
                scope_stack = null;
                break;
            case "jump_to_scope":
                scope_stack = scope_stack?.tail;
                break;
            case "push_scoped_symbol":
                const scopes = { scope: node.scope, tail: scope_stack };
                symbol_stack = { symbol: node.symbol, scopes, tail: symbol_stack };
                break;
            case "push_symbol":
                symbol_stack = { symbol: node.symbol, tail: symbol_stack };
                break;
            case "pop_scoped_symbol":
                scope_stack = symbol_stack?.scopes;
                symbol_stack = symbol_stack?.tail;
                break;
            case "pop_symbol":
                symbol_stack = symbol_stack?.tail;
                break;
            case "root":
            case "scope":
                break;
            default:
                console.log("Unknown node type ", node.type);
                break;
        }
        return [symbol_stack, scope_stack];
    }

    render() {
        // define svg
        const svg = container.append('svg')
            .attr('width', '100%')
            .attr('height', '100%');
        const background = svg.append("rect")
            .attr("class", "sg-background");
        this.sg = svg.append('g').attr('class', 'sg');

        // render UI
        this.render_help();
        this.render_tooltip();
        this.render_legend();
        this.render_graph();

        // pan & zoom
        let zoom = d3.zoom()
            .on('start', (e) => {
                background.classed("engaged", true);
            }).on('zoom', (e) => {
                this.sg.attr('transform', e.transform);
            }).on('end', (e) => {
                background.classed("engaged", false);
            });
        background.call(zoom);

        // global key events
        d3.select(window).on("keyup", (e) => {
            this.paths_keypress(e);
            this.tooltip_keypress(e);
            this.help_keypress(e);
        })
    }

    // ------------------------------------------------------------------------------------------------
    // Node Rendering
    //

    render_graph() {
        let that = this;

        // clear out the graph
        this.sg.selectAll('*').remove();

        const edge_group = this.sg.append("g");
        const node_group = this.sg.append("g");

        const connect = d3.dagConnect()
            .sourceId((edge) => this.ID[this.node_id_to_str(edge.source)])
            .targetId((edge) => this.ID[this.node_id_to_str(edge.sink)])
            .decycle(true);
        const dag = connect(this.graph.edges);

        // plot nodes
        const nodes = node_group
            .selectAll("g")
            .data(dag.descendants())
            .enter()
            .append("g");
        nodes.each(function (d, idx, gs) {
            that.render_node(that.N[d.data.id], d3.select(this));
        });
        nodes.each(function (d, idx, gs) {
            const bbox = this.getBBox({ fill: true, stroke: true });
            d.width = bbox.width;
            d.height = bbox.height;
        });

        const layout = d3.sugiyama()
            .nodeSize((d) => {
                return d === undefined ? [0, 0] : [d.width + 2 * StackGraph.distx, d.height + 2 * StackGraph.disty];
            });
        const { width, height } = layout(dag);

        // set viewport
        this.sg.attr("viewBox", [0, 0, width, height].join(" "));

        // plot edges
        const line = d3.line()
            .curve(d3.curveCatmullRom)
            .x((d) => d.x)
            .y((d) => d.y);
        const edges = edge_group
            .selectAll("path")
            .data(dag.links())
            .enter()
            .append("g")
            .attr("class", (d) => `${d.data.is_jump ? "jump" : "edge"} ${this.edge_to_file_class(d.data)}`)
            .attr("id", (d) => this.edge_to_id_str(d.data));
        edges.append("path")
            .attr("id", (d) => this.edge_to_id_str(d.data) + ":path")
            .attr("d", (d) => line(d.reversed ? d3.reverse(d.points) : d.points))
        let edge_labels = edges.append("text")
            .append("textPath")
            .attr("xlink:href", (d) => `#${this.edge_to_id_str(d.data)}:path`)
            .attr("startOffset", "45%")
            .text("➤");

        // position nodes
        nodes
            .attr("transform", ({ x, y, width, height }) => `translate(${x + StackGraph.margin - width / 2}, ${y - StackGraph.margin + height / 2})`);

        // node mouse events
        nodes
            .on("mouseover", (e, d) => {
                const node = this.N[d.data.id];
                this.current_node = node;
                this.node_focus(node);
                this.paths_mouseover(e, node);
                this.tooltip_mouseover(e);
            })
            .on("mousemove", (e, d) => {
                const node = this.N[d.data.id];
                this.tooltip_mousemove(e);
            })
            .on("mouseout", (e, d) => {
                const node = this.N[d.data.id];
                this.current_node = null;
                this.tooltip_mouseout(e);
                this.paths_mouseout(e, node);
                this.node_defocus(node);
            })
            .on("click", (e, d) => {
                const node = this.N[d.data.id];
                this.paths_click(e, node);
            });

        // edge mouse events
        edge_labels
            .on("mouseover", (e, d) => {
                let edge = d.data;
                this.current_edge = edge;
                this.tooltip_mouseover(e);
            })
            .on("mousemove", (e, d) => {
                let edge = d.data;
                this.tooltip_mousemove(e);
            })
            .on("mouseout", (e, d) => {
                let edge = d.data;
                this.current_edge = null;
                this.tooltip_mouseout(e);
            });

    }

    render_node(node, g) {
        g.attr('id', this.node_to_id_str(node));
        g.attr('class', `node ${node.type} ${this.node_to_file_class(node)}`);

        switch (node.type) {
            case "drop_scopes":
                this.render_symbol_node(g, "[drop]", null, "");
                break;
            case "jump_to_scope":
                this.render_symbol_node(g, "[jump]", null, "");
                break;
            case "pop_symbol":
                this.render_symbol_node(g, node.symbol, null, "pop");
                if (node.is_definition) {
                    g.classed('definition', true);
                }
                break;
            case "pop_scoped_symbol":
                let pop_scope = { class: "pop_scope" };
                this.render_symbol_node(g, node.symbol, pop_scope, "pop");
                if (node.is_definition) {
                    g.classed('definition', true);
                }
                break;
            case "push_symbol":
                this.render_symbol_node(g, node.symbol, null, "push");
                if (node.is_reference) {
                    g.classed('reference', true);
                }
                break;
            case "push_scoped_symbol":
                let push_scope = { class: "push_scope" };
                this.render_symbol_node(g, node.symbol, push_scope, "push");
                if (node.is_reference) {
                    g.classed('reference', true);
                }
                break;
            case "root":
                this.render_symbol_node(g, "[root]", null, "");
                break;
            case "scope":
                if (this.show_all_node_labels()) {
                    let v = '';
                    let l = '';
                    for (let i = 0; i < node.debug_info.length; i++) {
                        let info = node.debug_info[i];
                        if (info.key == "tsg_variable") {
                            v = info.value;
                        } else if (info.key == "tsg_location") {
                            l = info.value
                        }
                    }
                    this.render_symbol_node(g, v + " " + l);
                    g.classed('plain_labeled_node', true);
                } else {
                    this.render_scope(g);
                }
                if (node.is_exported) {
                    g.classed('exported', true);
                }
                break;
        }
    }

    render_symbol_node(g, text, scope, shape) {
        let content = g.append("g");
        content.append('text').text(text);
        let text_bbox = content.node().getBBox();
        if (scope !== undefined && scope !== null) {
            content.append("circle")
                .attr("class", scope.class)
                .attr("transform", `translate(${text_bbox.width + StackGraph.margin}, ${6 - text_bbox.height / 2})`);
            content.append("circle")
                .attr("class", scope.class + "-focus-point")
                .attr("transform", `translate(${text_bbox.width + StackGraph.margin}, ${6 - text_bbox.height / 2})`);
        }
        let bbox = content.node().getBBox();
        let l = bbox.x - StackGraph.margin,
            r = bbox.x + bbox.width + StackGraph.margin,
            t = bbox.y - StackGraph.margin,
            b = bbox.y + bbox.height + StackGraph.margin;
        var box_points;
        var arrow_points = null;
        switch (shape) {
            case "pop":
                box_points = `
                    ${l},${t}
                    ${r},${t}
                    ${r},${b}
                    ${l - StackGraph.arrow_stem_w},${b}
                    ${l - StackGraph.arrow_stem_w},${t + StackGraph.arrow_head_h}
                    ${l - StackGraph.arrow_head_w/2},${t + StackGraph.arrow_head_h}
                `;
                arrow_points = `
                    ${l},${t}
                    ${l + StackGraph.arrow_head_w/2},${t + StackGraph.arrow_head_h}
                    ${l + StackGraph.arrow_stem_w},${t + StackGraph.arrow_head_h}
                    ${l + StackGraph.arrow_stem_w},${b}
                    ${l - StackGraph.arrow_stem_w},${b}
                    ${l - StackGraph.arrow_stem_w},${t + StackGraph.arrow_head_h}
                    ${l - StackGraph.arrow_head_w/2},${t + StackGraph.arrow_head_h}
                `;
                break;
            case "push":
                box_points = `
                    ${l - StackGraph.arrow_stem_w},${t}
                    ${r},${t}
                    ${r},${b}
                    ${l},${b}
                    ${l - StackGraph.arrow_head_w/2},${b - StackGraph.arrow_head_h}
                    ${l - StackGraph.arrow_stem_w},${b - StackGraph.arrow_head_h}
                `;
                arrow_points = `
                    ${l - StackGraph.arrow_stem_w},${t}
                    ${l + StackGraph.arrow_stem_w},${t}
                    ${l + StackGraph.arrow_stem_w},${b - StackGraph.arrow_head_h}
                    ${l + StackGraph.arrow_head_w/2},${b - StackGraph.arrow_head_h}
                    ${l},${b}
                    ${l - StackGraph.arrow_head_w/2},${b - StackGraph.arrow_head_h}
                    ${l - StackGraph.arrow_stem_w},${b - StackGraph.arrow_head_h}
                `;
                break;
            default:
                box_points = `
                    ${l},${t}
                    ${r},${t}
                    ${r},${b}
                    ${l},${b}
                `;
                break;
        }
        if (arrow_points !== null) {
            g.append('polygon').lower()
                .attr("class", "arrow")
                .attr('points', arrow_points);
        }
        g.append('polygon').lower()
            .attr("class", "background")
            .attr('points', box_points);
        g.append('polygon').lower()
            .attr("class", "border")
            .attr('points', box_points);
    }

    render_scope(g) {
        g.append('circle')
            .attr("class", "border");
        g.append('circle')
            .attr("class", "background");
        g.append('circle')
            .attr("class", "focus-point");
    }

    // ------------------------------------------------------------------------------------------------
    // Node Highlighting
    //

    node_focus(node) {
        d3.select(this.id_selector(this.node_id_to_str(node.id)))
            .classed("focus", true);
        if (node.hasOwnProperty("scope")) {
            d3.select(this.id_selector(this.node_id_to_str(node.scope)))
                .classed("ref-focus", true);
        }
    }

    node_defocus(node) {
        d3.select(this.id_selector(this.node_id_to_str(node.id)))
            .classed("focus", false);
        if (node.hasOwnProperty("scope")) {
            d3.select(this.id_selector(this.node_id_to_str(node.scope)))
                .classed("ref-focus", false);
        }
    }

    // ------------------------------------------------------------------------------------------------
    // Path Highlighting
    //

    paths_mouseover(e, node) {
        if (this.paths_lock !== null) {
            return;
        }
        this.paths_highlight(node);
    }

    paths_mouseout(e, node) {
        if (this.paths_lock !== null) {
            return;
        }
        this.paths_nolight(node);
    }

    paths_click(e, node) {
        if (this.paths_lock === null) {
            if (node.paths.length > 0) {
                this.paths_nolight(node);
                this.paths_lock = { node, path: 0 };
                this.paths_highlight(node, 0);
                this.tooltip_update();
            }
        } else if (this.paths_lock.node === node) {
            this.paths_nolight(node, this.paths_lock.path);
            this.paths_lock.path += 1;
            if (this.paths_lock.path >= node.paths.length) {
                this.paths_lock = null;
                this.paths_highlight(node);
            } else {
                this.paths_highlight(node, this.paths_lock.path);
            }
            this.tooltip_update();
        }
    }

    paths_keypress(e) {
        if (this.paths_lock !== null) {
            if (e.keyCode === 27) {
                this.paths_nolight(this.paths_lock.node);
                this.node_defocus(this.paths_lock.node);
                this.paths_lock = null;
                if (this.current_node !== null) {
                    this.node_focus(this.current_node);
                    this.paths_highlight(this.current_node);
                    this.tooltip_update();
                }
            } else if (e.keyCode == 78) {
                this.paths_nolight(this.paths_lock.node, this.paths_lock.path);
                this.paths_lock.path += 1;
                if (this.paths_lock.path >= this.paths_lock.node.paths.length) {
                    this.paths_lock.path = 0;
                }
                this.paths_highlight(this.paths_lock.node, this.paths_lock.path);
                if (this.current_node !== null) {
                    this.tooltip_update();
                }
            }
        }
    }

    paths_highlight(node, path) {
        const paths = (path !== undefined) ? [node.paths[path]] : node.paths;
        const nodes = {};
        const edges = {};
        for (let path of paths) {
            for (let node_id in path.derived.nodes) {
                if (!nodes.hasOwnProperty(node_id)) {
                    nodes[node_id] = false;
                }
            }
            if (path.edges.length > 0) {
                nodes[this.node_id_to_str(path.start_node)] = true;
                nodes[this.node_id_to_str(path.end_node)] = true;
            }
            for (let edge_id in path.derived.edges) {
                if (!edges.hasOwnProperty(edge_id)) {
                    edges[edge_id] = 0;
                }
                edges[edge_id] += 1;
            }
            for (let node_id in nodes) {
                const g = d3.select(this.id_selector(node_id));
                g.classed("path-node", true);
                if (nodes[node_id]) {
                    g.classed("path-endpoint", true);
                }
            }
            for (let edge_id in edges) {
                const g = d3.select(this.id_selector(edge_id));
                g.classed("path-edge", true);
            }
        }
    }

    paths_nolight(node, path) {
        const paths = (path !== undefined) ? [node.paths[path]] : node.paths;
        for (let path of paths) {
            for (let node_id in path.derived.nodes) {
                const g = d3.select(this.id_selector(node_id));
                g.classed("path-node", false);
                g.classed("path-endpoint", false);
            }
            for (let edge_id in path.derived.edges) {
                const g = d3.select(this.id_selector(edge_id));
                g.classed("path-edge", false);
            }
        }
    }

    // ------------------------------------------------------------------------------------------------
    // Tooltip
    //

    render_tooltip() {
        d3.select('body').append('div')
            .attr('id', 'sg-tooltip')
            .classed(this.tooltip_orient_class(), true);
    }

    tooltip_keypress(e) {
        const old_class = this.tooltip_orient_class();
        switch (e.keyCode) {
            case 87: // w
                this.current_orient.y = "north";
                break;
            case 65: // a
                this.current_orient.x = "west";
                break;
            case 83: // s
                this.current_orient.y = "south";
                break;
            case 68: // d
                this.current_orient.x = "east";
                break;
        }
        const new_class = this.tooltip_orient_class();
        d3.select('#sg-tooltip')
            .classed(old_class, false)
            .classed(new_class, true);
    }

    tooltip_orient_class() {
        return `${this.current_orient.y}-${this.current_orient.x}`;
    }

    tooltip_mousemove(e, node) {
        d3.select('#sg-tooltip')
            .style('left', `${e.pageX}px`)
            .style('top', `${e.pageY}px`);
    }

    tooltip_mouseover(e) {
        this.tooltip_update();
    }

    tooltip_mouseout(e) {
        this.tooltip_update();
    }

    tooltip_update() {
        const tooltip = d3.select('#sg-tooltip');

        if (!this.tooltip_visible() || (this.current_node === null && this.current_edge === null)) {
            tooltip.style('visibility', 'hidden');
            return;
        }

        // clear
        tooltip.selectAll("*").remove();

        // create table
        const tbody = tooltip.append("table")
            .attr("class", "sg-tooltip-table")
            .append("tbody");
        function add_header(label) {
            const tr = tbody.append("tr")
                .attr("class", "sg-tooltip-header");
            tr.append("td")
                .attr("colspan", "2")
                .text(label);
        }
        function add_sub_header(label) {
            const tr = tbody.append("tr")
                .attr("class", "sg-tooltip-sub-header");
            tr.append("td")
                .attr("colspan", "2")
                .text(label);
        }
        function add_row(label, value) {
            const tr = tbody.append("tr");
            tr.append("td").attr("class", "sg-tooltip-label").text(label);
            const td = tr.append("td").attr("class", "sg-tooltip-value")
            if (Array.isArray(value)) {
                const ul = td.append("ul").attr("class", "sg-tooltip-list");
                for (let element of value) {
                    const li = ul.append("li").attr("class", "sg-tooltip-list-element");
                    if (Array.isArray(element)) {
                        const subvalue = element[0];
                        const sublist = element[1];
                        li.append("span").text(subvalue);
                        let sub_ul = li.append("ul").attr("class", "sg-tooltip-sublist");
                        for (let sub_element of sublist) {
                            sub_ul.append("li").attr("class", "sg-tooltip-sublist-element").text(sub_element);
                        }
                    } else {
                        li.text(element);
                    }
                }
            } else {
                td.text(value);
            }
        }
        let tooltip_methods = {
            add_header,
            add_sub_header,
            add_row,
        };

        if (this.current_node != null) {
            this.tooltip_node_update(tooltip_methods, this.current_node);
            if (this.paths_lock !== null) {
                this.tooltip_path_update(tooltip_methods, this.paths_lock);
            }
            tooltip.style('visibility', 'visible');
        } else if (this.current_edge != null) {
            this.tooltip_edge_update(tooltip_methods, this.current_edge);
            if (this.paths_lock !== null) {
                this.tooltip_path_update(tooltip_methods, this.paths_lock);
            }
            tooltip.style('visibility', 'visible');
        }
    }

    tooltip_edge_update(tooltip, edge) {
        tooltip.add_header("edge info");
        tooltip.add_row("source", this.node_id_to_str(edge.source));
        tooltip.add_row("sink", this.node_id_to_str(edge.sink));
        if (edge.hasOwnProperty("precedence")) {
            tooltip.add_row("precedence", edge.precedence);
        }

        if (edge.hasOwnProperty("debug_info") && edge.debug_info.length > 0) {
            tooltip.add_header("debug info");
            for (let { key, value } of edge.debug_info.sort((l, r) => l.key > r.key)) {
                tooltip.add_row(key, value);
            }
        }
    }

    tooltip_node_update(tooltip, node) {
        tooltip.add_header("node info");
        tooltip.add_row("id", this.node_to_id_str(node));
        tooltip.add_row("type", node.type);
        if (node.hasOwnProperty("scope")) {
            tooltip.add_row("scope", this.node_id_to_str(node.scope));
        }
        if (node.hasOwnProperty("is_reference")) {
            tooltip.add_row("reference?", node.is_reference ? "yes" : "no");
        }
        if (node.hasOwnProperty("is_definition")) {
            tooltip.add_row("definition?", node.is_definition ? "yes" : "no");
        }
        if (node.hasOwnProperty("is_exported")) {
            tooltip.add_row("exported?", node.is_exported ? "yes" : "no");
        }
        if (this.node_has_source_info(node)) {
            if (!this.span_is_empty(node.source_info.span)) {
                tooltip.add_row("location", this.location_to_str(node.source_info.span.start));
            }
            if (node.source_info.syntax_type) {
                tooltip.add_row("syntax type", node.source_info.syntax_type);
            }
        }
        if (node.paths.length > 0) {
            tooltip.add_row("outgoing paths", `${node.paths.length}`);
        }

        if (node.hasOwnProperty("debug_info") && node.debug_info.length > 0) {
            tooltip.add_header("debug info");
            for (let { key, value } of node.debug_info.sort((l, r) => l.key > r.key)) {
                tooltip.add_row(key, value);
            }
        }
    }

    tooltip_path_update(tooltip, paths_lock) {
        if (!this.tooltip_on_current_path(paths_lock)) {
            return;
        }
        let path = paths_lock.node.paths[paths_lock.path];
        tooltip.add_header("path info");
        const path_count = `(path ${paths_lock.path + 1} of ${paths_lock.node.paths.length})`;
        tooltip.add_row("start node", `${this.node_id_to_str(path.start_node)} ${path_count}`);
        tooltip.add_row("end node", `${this.node_id_to_str(path.end_node)}`);

        const node_id = (this.current_node && this.node_to_id_str(this.current_node))
            || (this.current_edge && this.node_id_to_str(this.current_edge.source));
        const node_data = path.derived.nodes[node_id];
        for (const { index, symbol_stack, scope_stack } of node_data.stacks) {
            tooltip.add_sub_header(`position ${index}`);
            tooltip.add_row("symbol stack", this.symbol_stack_to_array(symbol_stack));
            tooltip.add_row("scope stack", this.scope_stack_to_array(scope_stack));
        }
    }

    tooltip_on_current_path(paths_lock) {
        let path = paths_lock.node.paths[paths_lock.path];
        return (this.current_node !== null && path.derived.nodes.hasOwnProperty(this.node_to_id_str(this.current_node)))
            || (this.current_edge !== null && path.derived.edges.hasOwnProperty(this.edge_to_id_str(this.current_edge)));
    }

    // ------------------------------------------------------------------------------------------------
    // Legend
    //

    render_legend() {
        const legend = d3.select('body').append('div')
            .attr('id', 'sg-legend')
        legend.append("h1").text("Files");
        const items = legend.append("ul");
        items.append("li")
            .classed("global", true)
            .text("[global]");
        for (const file in this.F) {
            items.append("li")
                .classed('file-' + this.F[file], true)
                .text(file);
        }
    }

    legend_update() {
        const legend = d3.select('#sg-legend');
        legend.style('visibility', this.show_file_legend() ? null : 'hidden');
    }

    // ------------------------------------------------------------------------------------------------
    // Help
    //

    render_help() {
        const help = d3.select('body').append('div');
        help.append('label')
            .attr('for', 'sg-help-toggle')
            .attr('class', 'sg-help-label')
            .text("ⓘ");
        this.help_toggle = help.append('input')
            .attr('id', 'sg-help-toggle')
            .attr('type', 'checkbox');
        const help_content = help.append('div')
            .attr('class', 'sg-help-content');

        help_content.append("h1").text("Graph");
        help_content.append("p").html(`
            Pan by dragging the background with the mouse.
            Zoom using the scroll wheel.
        `);
        this.show_files_legend_toggle = this.new_setting(help_content, "sg-files-legend", "Show files legend (<kbd>f</kbd>)", true);
        this.show_files_legend_toggle.on("change", (e => {
            this.legend_update();
        }));
        this.show_all_node_labels_toggle = this.new_setting(help_content, "sg-scope-labels", "Show all node labels (<kbd>l</kbd>)", false);
        this.show_all_node_labels_toggle.on("change", (e => {
            this.render_graph();
        }));

        help_content.append("h1").text("Nodes & Edges");
        help_content.append("p").html(`
            Hover over nodes and edges to get a tooltip with detailed information.
            Change the tooltip orientation using the keys <kbd>w</kbd> for above, <kbd>a</kbd> for left of, <kbd>s</kbd> for below, or <kbd>d</kbd> for right of the pointer.
        `);
        this.tooltip_toggle = this.new_setting(help_content, "sg-tooltip-visibility", "Show tooltip (<kbd>v</kbd>)", true);
        this.tooltip_toggle.on("change", (e => {
            this.tooltip_update();
        }));

        help_content.append("h1").text("Paths");
        help_content.append("p").html(`
            Cycle through individual paths by clicking on a node with outgoing paths.
            While a path is selected, clicks to other nodes than the source node have no effect.
            Cycle through selected paths using the key <kbd>n</kbd>.
            Path selection ends after cycling through all paths by clicking the node, or by pressing the <kbd>esc</kbd> key.
        `);

        help_content.append("p").attr("class", "sg-help-meta").html(`
            Toggle visibility of this help anytime by pressing <kbd>h</kbd>.
        `);

        const byline = help_content.append('div')
            .attr("class", "sg-help-byline");
        if (this.metadata?.version) {
            byline.text(this.metadata.version);
        }
    }

    help_keypress(e) {
        switch (e.keyCode) {
            case 70: // f
                this.show_files_legend_toggle.property("checked", !this.show_files_legend_toggle.property("checked"));
                this.legend_update();
                break;
            case 72: // h
                this.help_toggle.property("checked", !this.help_toggle.property("checked"));
                break;
            case 76: // h
                this.show_all_node_labels_toggle.property("checked", !this.show_all_node_labels_toggle.property("checked"));
                this.render_graph();
                break;
            case 86: // v
                this.tooltip_toggle.property("checked", !this.tooltip_visible());
                this.tooltip_update();
                break;
        }
    }

    tooltip_visible() {
        return this.tooltip_toggle.property("checked");
    }

    show_all_node_labels() {
        return this.show_all_node_labels_toggle.property("checked");
    }

    show_file_legend() {
        return this.show_files_legend_toggle.property("checked");
    }

    new_setting(element, id, html, initial) {
        const toggle = element.append("div");
        const toggle_input = toggle.append('input')
            .attr('id', id)
            .attr('type', 'checkbox')
            .attr('class', 'sg-toggle-input')
            .property('checked', initial);
        toggle.append('label')
            .attr('for', id)
            .attr('class', 'sg-toggle-label')
            .html(html);
        return toggle_input;
    }

    // ------------------------------------------------------------------------------------------------
    // Node & Edge IDs
    //

    node_to_id_str(node) {
        return this.node_id_to_str(node.id);
    }

    node_to_file_class(node) {
        return this.node_id_to_file_class(node.id);
    }

    edge_to_id_str(edge) {
        return this.node_id_to_str(edge.source) + "->" + this.node_id_to_str(edge.sink);
    }

    edge_to_file_class(edge) {
        if (edge.source.hasOwnProperty('file')) {
            return "file-" + (this.F[edge.source.file] % StackGraph.number_of_file_colors);
        } else if (edge.sink.hasOwnProperty('file')) {
            return "file-" + (this.F[edge.sink.file] % StackGraph.number_of_file_colors);
        } else {
            return "global";
        }
    }

    node_id_to_str(id) {
        if (id.hasOwnProperty('file')) {
            return id.file + "#" + id.local_id;
        } else {
            return "#" + id.local_id;
        }
    }

    node_id_to_file_class(id) {
        if (id.hasOwnProperty('file')) {
            return "file-" + (this.F[id.file] % StackGraph.number_of_file_colors);
        } else {
            return "global";
        }
    }

    id_selector(id) {
        const sel = "#" + id.replaceAll(/[^a-zA-Z0-9]/g, '\\$&');
        return sel;
    }

    // ------------------------------------------------------------------------------------------------
    // Source Info
    //

    node_has_source_info(node) {
        return node.hasOwnProperty("source_info")
    }

    span_to_str(span) {
        return `${this.location_to_str(span.start)}–${this.location_to_str(span.end)}`;
    }

    location_to_str(loc) {
        return `${loc.line + 1}:${loc.column.grapheme_offset + 1}`;
    }

    span_is_empty(span) {
        return !span
            || (    span.start.line === 0
                 && span.start.column.utf8_offset === 0
                 && span.end.line === 0
                 && span.end.column.utf8_offset === 0
               );
    }

    // ------------------------------------------------------------------------------------------------
    // Stacks
    //

    symbol_stack_to_array(symbol_stack) {
        let result = [];
        while (symbol_stack !== null) {
            let symbol = symbol_stack.symbol;
            if (symbol_stack.scopes) {
                const scopes = this.scope_stack_to_array(symbol_stack.scopes);
                symbol = [symbol, scopes];
            }
            result.push(symbol);
            symbol_stack = symbol_stack.tail;
        }
        return result;
    }

    scope_stack_to_array(scope_stack) {
        let result = [];
        while (scope_stack !== null) {
            result.push(this.node_id_to_str(scope_stack.scope));
            scope_stack = scope_stack.tail;
        }
        return result;
    }

}
