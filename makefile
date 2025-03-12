CC = gcc
OPENSSL_PATH = $(shell brew --prefix openssl@3)
CFLAGS = -Wall -Wextra -I$(OPENSSL_PATH)/include
LDFLAGS = -L$(OPENSSL_PATH)/lib -lssl -lcrypto

vcs: vcs.o
	$(CC) -o vcs vcs.o $(LDFLAGS)

vcs.o: vcs.c vcs.h
	$(CC) $(CFLAGS) -c vcs.c

clean:
	rm -f vcs vcs.o
