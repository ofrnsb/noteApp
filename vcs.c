#include "vcs.h"

int main(int argc, char *argv[]) {
    if (argc < 2) {
        printf("Usage: vcs <command> [args]\n");
        return 1;
    }

    if (strcmp(argv[1], "init") == 0) {
        init_vcs();
    } 
    else if (strcmp(argv[1], "add") == 0) {
        if (argc < 3) {
            printf("Usage: vcs add <filename> or vcs add .\n");
            return 1;
        }
        if (strcmp(argv[2], ".") == 0) {
            add_all_files();
        } else {
            add_file(argv[2]);
        }
    } 
    else if (strcmp(argv[1], "history") == 0) {
        show_history();
    }
    else {
        printf("Unknown command: %s\n", argv[1]);
    }

    return 0;
}

void init_vcs() {
    mkdir(".vcs", 0777);
    mkdir(".vcs/objects", 0777);
    printf("Initialized empty VCS repository in .vcs/\n");
}

char *compute_hash(const char *filename) {
    FILE *file = fopen(filename, "rb");
    if (!file) {
        perror("Error opening file");
        return NULL;
    }

    EVP_MD_CTX *mdctx = EVP_MD_CTX_new();
    if (!mdctx) {
        perror("Error creating EVP context");
        fclose(file);
        return NULL;
    }

    unsigned char buffer[MAX_BUFFER];
    unsigned char hash[EVP_MAX_MD_SIZE];
    unsigned int hash_len;

    EVP_DigestInit_ex(mdctx, EVP_sha256(), NULL);
  
    int bytesRead;
    while ((bytesRead = fread(buffer, 1, MAX_BUFFER, file)) > 0) {
        EVP_DigestUpdate(mdctx, buffer, bytesRead);
    }
  
    EVP_DigestFinal_ex(mdctx, hash, &hash_len);
    EVP_MD_CTX_free(mdctx);
    fclose(file);

    static char hashStr[MAX_HASH_LEN];
    for (unsigned int i = 0; i < hash_len; i++) {
        sprintf(hashStr + (i * 2), "%02x", hash[i]);
    }
    hashStr[hash_len * 2] = '\0';

    return hashStr;
}

void save_file_snapshot(const char *filename, const char *hash) {
    char subdir[3] = {hash[0], hash[1], '\0'};
    char dir_path[MAX_BUFFER];
    char file_path[MAX_BUFFER];
    sprintf(dir_path, ".vcs/objects/%s", subdir);
    sprintf(file_path, ".vcs/objects/%s/%s", subdir, hash + 2);

    struct stat st = {0};
    if (stat(dir_path, &st) == -1) {
        mkdir(dir_path, 0777);
    }

    FILE *source = fopen(filename, "rb");
    FILE *dest = fopen(file_path, "wb");
    if (!source || !dest) {
        perror("Error saving snapshot");
        if (source) fclose(source);
        if (dest) fclose(dest);
        return;
    }

    char buffer[MAX_BUFFER];
    int bytesRead;
    while ((bytesRead = fread(buffer, 1, MAX_BUFFER, source)) > 0) {
        fwrite(buffer, 1, bytesRead, dest);
    }

    fclose(source);
    fclose(dest);
}

void add_file(const char *filename) {
    char *hash = compute_hash(filename);
    if (!hash) return;

    save_file_snapshot(filename, hash);
    printf("Added %s with hash %s\n", filename, hash);
}

void create_commit() {
    FILE *index = fopen(".vcs/index", "r");
    if (!index) {
        printf("No files staged for commit\n");
        return;
    }

    char commit_id[20];
    sprintf(commit_id, "%ld", (long)time(NULL));

    char commit_path[MAX_BUFFER];
    sprintf(commit_path, ".vcs/commits/%s", commit_id);
    FILE *commit_file = fopen(commit_path, "w");
    if (!commit_file) {
        mkdir(".vcs/commits", 0777);
        commit_file = fopen(commit_path, "w");
    }

    char line[MAX_BUFFER];
    while (fgets(line, MAX_BUFFER, index)) {
        char *file = strtok(line, " ");
        char *hash = strtok(NULL, "\n");
        if (file && hash) {
            fprintf(commit_file, "%s %s\n", file, hash);
        }
    }

    fclose(index);
    fclose(commit_file);
    printf("Created commit %s\n", commit_id);
}

void add_all_files() {
    DIR *dir = opendir(".");
    if (!dir) {
        perror("Error opening directory");
        return;
    }

    FILE *index = fopen(".vcs/index", "w");
    if (!index) {
        index = fopen(".vcs/index", "w");
    }

    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || 
            strcmp(entry->d_name, "..") == 0 || 
            strcmp(entry->d_name, ".vcs") == 0) {
            continue;
        }

        struct stat path_stat;
        stat(entry->d_name, &path_stat);
        if (S_ISREG(path_stat.st_mode)) {
            char *hash = compute_hash(entry->d_name);
            if (hash) {
                save_file_snapshot(entry->d_name, hash);
                fprintf(index, "%s %s\n", entry->d_name, hash);
                printf("Added %s with hash %s\n", entry->d_name, hash);
            }
        }
    }

    fclose(index);
    closedir(dir);
    create_commit();
}

void show_history() {
    DIR *commits_dir = opendir(".vcs/commits");
    if (!commits_dir) {
        printf("No commit history found\n");
        return;
    }

    // Kumpulkan semua commit untuk diurutkan
    char commit_ids[1000][20];
    int commit_count = 0;

    struct dirent *entry;
    while ((entry = readdir(commits_dir)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || 
            strcmp(entry->d_name, "..") == 0) {
            continue;
        }
        strcpy(commit_ids[commit_count++], entry->d_name);
    }
    closedir(commits_dir);

    // Urutkan commit berdasarkan ID (timestamp)
    for (int i = 0; i < commit_count - 1; i++) {
        for (int j = i + 1; j < commit_count; j++) {
            if (strcmp(commit_ids[i], commit_ids[j]) > 0) {
                char temp[20];
                strcpy(temp, commit_ids[i]);
                strcpy(commit_ids[i], commit_ids[j]);
                strcpy(commit_ids[j], temp);
            }
        }
    }

    // Tampilkan riwayat per commit
    for (int i = 0; i < commit_count; i++) {
        char commit_path[MAX_BUFFER];
        sprintf(commit_path, ".vcs/commits/%s", commit_ids[i]);
        FILE *commit_file = fopen(commit_path, "r");
        if (!commit_file) continue;

        time_t commit_time = atol(commit_ids[i]); // Konversi string ke time_t
        char *time_str = ctime(&commit_time);
        if (!time_str) time_str = "Unknown date\n";

        printf("\nCommit %s\n", commit_ids[i]);
        printf("Date: %s", time_str);

        char line[MAX_BUFFER];
        while (fgets(line, MAX_BUFFER, commit_file)) {
            char *file = strtok(line, " ");
            char *hash = strtok(NULL, "\n");
            if (file && hash) {
                printf("  %s: %s\n", file, hash);
            }
        }
        fclose(commit_file);
    }
}