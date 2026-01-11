
CC=clang
CFLAGS=-Wall --target=wasm32-unknown-unknown -ffreestanding -nostdlib -mbulk-memory -O3

SRCDIR=src
OUTDIR=target
OBJDIR=$(OUTDIR)/build
LIBDIR=$(OUTDIR)/sysroot/lib
INCDIR=$(OUTDIR)/sysroot/include
OUTPUT=$(LIBDIR)/libc.a
CRTO=$(LIBDIR)/crt1.o

SRC_LIBC = ctype.o math.o stdio.o stdlib.o string.o stat.o errno.o
OBJS += $(addprefix $(OBJDIR)/, $(SRC_LIBC))

sysroot: $(OUTPUT) $(CRTO) | $(INCDIR)
	cp $(SRCDIR)/*.h $(INCDIR)/
	cp $(SRCDIR)/sys/*.h $(INCDIR)/sys/

clean:
	rm -rf $(OUTDIR)

$(OUTPUT): $(OBJS) | $(LIBDIR)
	llvm-ar rcs $(OUTPUT) $(OBJS)

$(OBJS): | $(OBJDIR)

$(OBJDIR):
	mkdir -p $(OBJDIR)

$(LIBDIR):
	mkdir -p $(LIBDIR)

$(INCDIR):
	mkdir -p $(INCDIR)/sys

$(OBJDIR)/%.o:	$(SRCDIR)/%.c
	$(CC) $(CFLAGS) -c $< -o $@

$(CRTO): $(SRCDIR)/crt1.c | $(LIBDIR)
	$(CC) $(CFLAGS) -c $< -o $@
