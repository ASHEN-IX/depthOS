/* DepthOS bounded EIS sender. Uses installed libei; no uinput or X11.
 * Receives only an already-authorized EIS fd from the runner.
 * Input is released before closing; EOF/SIGTERM stops this bounded helper.
 */
#include <libei.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
static struct ei *ctx;
static struct ei_device *keyboard, *pointer;
static int kr, pr;
static volatile sig_atomic_t stop;
static unsigned char keys[256], buttons[16];
static void stopped(int sig) { (void)sig; stop=1; }
static uint64_t now(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC,&t); return (uint64_t)t.tv_sec*1000000+t.tv_nsec/1000; }
static void pump(void) {
    ei_dispatch(ctx);
    struct ei_event *event;
    while ((event=ei_get_event(ctx))) {
        enum ei_event_type type=ei_event_get_type(event);
        fprintf(stderr,"EIS event %d\n",type);
        if (type==EI_EVENT_SEAT_ADDED) {
            struct ei_seat *seat=ei_event_get_seat(event);
            ei_seat_bind_capabilities(seat,EI_DEVICE_CAP_POINTER_ABSOLUTE,EI_DEVICE_CAP_KEYBOARD,EI_DEVICE_CAP_BUTTON,NULL);
        } else if (type==EI_EVENT_DEVICE_ADDED) {
            struct ei_device *d=ei_event_get_device(event);
            fprintf(stderr,"device k=%d abs=%d button=%d\n",ei_device_has_capability(d,EI_DEVICE_CAP_KEYBOARD),ei_device_has_capability(d,EI_DEVICE_CAP_POINTER_ABSOLUTE),ei_device_has_capability(d,EI_DEVICE_CAP_BUTTON));
            if (ei_device_has_capability(d,EI_DEVICE_CAP_KEYBOARD)) keyboard=ei_device_ref(d);
            if (ei_device_has_capability(d,EI_DEVICE_CAP_POINTER_ABSOLUTE)) pointer=ei_device_ref(d);
        } else if (type==EI_EVENT_DEVICE_RESUMED) {
            struct ei_device *d=ei_event_get_device(event);
            ei_device_start_emulating(d,1);
            if (d==keyboard) kr=1;
            if (d==pointer) pr=1;
        } else if (type==EI_EVENT_DEVICE_REMOVED || type==EI_EVENT_DEVICE_PAUSED || type==EI_EVENT_DISCONNECT) {
            stop=1; /* fail closed; no device substitution during a test */
        }
        ei_event_unref(event);
    }
}
static void frame(struct ei_device *d) { ei_device_frame(d,now()); ei_dispatch(ctx); }
int main(int argc,char **argv) {
    if (argc!=2) return 2;
    signal(SIGTERM,stopped);signal(SIGINT,stopped);
    ctx=ei_new_sender(NULL);ei_configure_name(ctx,"DepthOS bounded acceptance");
    if (ei_setup_backend_fd(ctx,atoi(argv[1]))!=0) return 3;
    uint64_t until=now()+5000000;
    while (!stop && !(kr&&pr) && now()<until) { pump(); usleep(10000); }
    if (!(kr&&pr) || stop) { fprintf(stderr,"EIS devices not ready\n"); ei_unref(ctx);return 4; }
    setvbuf(stdout,NULL,_IOLBF,0);puts("READY");
    char line[160];
    while (!stop) {
        struct pollfd fds[2]={{STDIN_FILENO,POLLIN,0},{ei_get_fd(ctx),POLLIN,0}};
        if (poll(fds,2,100)<0) continue;
        if (fds[1].revents) pump();
        if (fds[0].revents & (POLLIN|POLLHUP)) {
            if (!fgets(line,sizeof(line),stdin)) break;
            unsigned key,value;double x,y;
            if (!strncmp(line,"quit",4)) break;
            if (sscanf(line,"key %u %u",&key,&value)==2 && key<256 && value<=1) {
                ei_device_keyboard_key(keyboard,key,value!=0);keys[key]=value;frame(keyboard);
            } else if (sscanf(line,"move %lf %lf",&x,&y)==2 && x>=0 && x<1920 && y>=0 && y<1080) {
                ei_device_pointer_motion_absolute(pointer,x,y);frame(pointer);
            } else if (sscanf(line,"button %u %u",&key,&value)==2 && key>=272 && key<288 && value<=1) {
                ei_device_button_button(pointer,key,value!=0);buttons[key-272]=value;frame(pointer);
            } else { puts("ERROR invalid bounded command");continue; }
            puts("OK");
        }
    }
    if (!stop || (kr&&pr)) {
        for (unsigned i=0;i<256;i++) if(keys[i]) ei_device_keyboard_key(keyboard,i,0);
        for (unsigned i=0;i<16;i++) if(buttons[i]) ei_device_button_button(pointer,i+272,0);
        if (keyboard) frame(keyboard);
        if(pointer) frame(pointer);
    }
    if (keyboard) ei_device_unref(keyboard);
    if(pointer) ei_device_unref(pointer);
    ei_unref(ctx);return 0;
}
