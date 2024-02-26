/*
 * gcc --std=c11 -o c-example ffi.c -L../target/debug/ -lmemerge
 * LD_LIBRARY_PATH=target/debug/ ./c-example
 */
    
#include <stdio.h>
#include <stdint.h>

#include "../include/memerge.h"

int main(void) {
  uint32_t sum = addition(1, 2);
  printf("%u\n", sum);
  return 0;
}
