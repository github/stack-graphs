SRCDIR  := src
HTMLDIR := html

SRCS  := $(wildcard $(SRCDIR)/*.md)
HTMLS := $(patsubst $(SRCDIR)/%.md,$(HTMLDIR)/%.html,$(SRCS))

$(HTMLDIR)/%.html: $(SRCDIR)/%.md $(HTMLDIR)
	@script/mdmath2html $< > $@

.PHONY: all install help

default: build-html

build-html: $(HTMLS)

$(HTMLDIR):
	mkdir -p $(HTMLDIR)

install:
	npm install

help:
	@echo "Supported targets:"
	@echo "    build-html (default)    Render .md files as HTML in '$(HTMLDIR)' directory"
	@echo "    install                 Install required NPM modules in this directory"
	@echo "    help                    Show this help"
