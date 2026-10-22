#include <jni.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
extern void *cm_open(const char *, const char *, uint16_t, bool);
extern char *cm_call(void *, const char *);
extern void cm_close(void *);
extern void cm_free(char *);
// JNI Modified UTF-8 is not Rust UTF-8. Transfer actual UTF-8 byte arrays.
static char *copy(JNIEnv *e,jbyteArray a) { jsize n=(*e)->GetArrayLength(e,a);char *s=malloc((size_t)n+1);if(!s)return NULL;(*e)->GetByteArrayRegion(e,a,0,n,(jbyte*)s);s[n]=0;return s; }
JNIEXPORT jlong JNICALL Java_org_campusmesh_Native_open(JNIEnv *e,jobject o,jbyteArray p,jbyteArray n) {char *path=copy(e,p),*nick=copy(e,n);void *h=path&&nick?cm_open(path,nick,37078,true):NULL;free(path);free(nick);return (jlong)(intptr_t)h;}
JNIEXPORT jbyteArray JNICALL Java_org_campusmesh_Native_call(JNIEnv *e,jobject o,jlong h,jbyteArray input) {char *s=copy(e,input);if(!s)return NULL;char *out=cm_call((void*)(intptr_t)h,s);free(s);jsize len=(jsize)strlen(out);jbyteArray a=(*e)->NewByteArray(e,len);if(a)(*e)->SetByteArrayRegion(e,a,0,len,(jbyte*)out);cm_free(out);return a;}
JNIEXPORT void JNICALL Java_org_campusmesh_Native_close(JNIEnv *e,jobject o,jlong h) {cm_close((void*)(intptr_t)h);}
