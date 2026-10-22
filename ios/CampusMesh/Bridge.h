#include <stdint.h>
#include <stdbool.h>
void *cm_open(const char *path,const char *nickname,uint16_t port,bool discovery);
char *cm_call(void *handle,const char *json);
void cm_close(void *handle);
void cm_free(char *value);
