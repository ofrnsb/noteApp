#ifndef VCS_H
#define VCS_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <openssl/evp.h>
#include <dirent.h>
#include <time.h>

#define MAX_HASH_LEN 65
#define MAX_BUFFER 1024

void init_vcs();
char *compute_hash(const char *filename);
void save_file_snapshot(const char *filename, const char *hash);
void add_file(const char *filename);
void add_all_files();
void show_history();
void create_commit();

#endif