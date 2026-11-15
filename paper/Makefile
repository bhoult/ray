# Build the paper: make   (needs pdflatex + bibtex; latexmk if available)
all: fractalkit.pdf
fractalkit.pdf: fractalkit.tex references.bib
	latexmk -pdf -interaction=nonstopmode fractalkit.tex
clean:
	latexmk -C
