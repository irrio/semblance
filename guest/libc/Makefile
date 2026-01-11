
CC=clang
CFLAGS=-Wall --target=wasm32-unknown-unknown -ffreestanding -nostdlib -mbulk-memory -O3

SRCDIR=src
OUTDIR=target
OBJDIR=$(OUTDIR)/build
VENDIR=$(OBJDIR)/vendor
LIBDIR=$(OUTDIR)/sysroot/lib
INCDIR=$(OUTDIR)/sysroot/include
OUTPUT=$(LIBDIR)/libc.a
CRTO=$(LIBDIR)/crt1.o

SRC_LIBC = ctype.o math.o stdio.o stdlib.o string.o stat.o errno.o
OBJS += $(addprefix $(OBJDIR)/, $(SRC_LIBC))

SRC_VENDOR = walloc.o
OBJS_VENDOR += $(addprefix $(VENDIR)/, $(SRC_VENDOR))

sysroot: $(OUTPUT) $(CRTO) | $(INCDIR)
	cp $(SRCDIR)/*.h $(INCDIR)/
	cp $(SRCDIR)/sys/*.h $(INCDIR)/sys/
	cp $(SRCDIR)/semblance/*.h $(INCDIR)/semblance/

clean:
	rm -rf $(OUTDIR)

$(OUTPUT): $(OBJS) $(OBJS_VENDOR) | $(LIBDIR)
	llvm-ar rcs $(OUTPUT) $(OBJS) $(OBJS_VENDOR)

$(OBJS): | $(OBJDIR)

$(OBJS_VENDOR): | $(VENDIR)

$(OBJDIR):
	mkdir -p $(OBJDIR)

$(VENDIR):
	mkdir -p $(VENDIR)

$(LIBDIR):
	mkdir -p $(LIBDIR)

$(INCDIR):
	mkdir -p $(INCDIR)/sys
	mkdir -p $(INCDIR)/semblance

$(OBJDIR)/%.o:	$(SRCDIR)/%.c
	$(CC) $(CFLAGS) -c $< -o $@

$(VENDIR)/%.o:	$(SRCDIR)/vendor/%.c
	$(CC) $(CFLAGS) -c $< -o $@

$(CRTO): $(SRCDIR)/crt1.c | $(LIBDIR)
	$(CC) $(CFLAGS) -c $< -o $@
