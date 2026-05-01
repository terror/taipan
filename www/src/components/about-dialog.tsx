import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTrigger,
} from '@/components/ui/dialog';

export const AboutDialog = () => {
  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant='ghost' size='sm' className='cursor-pointer'>
          about
        </Button>
      </DialogTrigger>

      <DialogContent className='sm:max-w-[520px]'>
        <DialogHeader>
          <DialogDescription>
            <span className='font-semibold'>taipan</span> is an experimental
            Python interpreter written in Rust.
          </DialogDescription>
        </DialogHeader>

        <div className='text-muted-foreground grid gap-4 text-sm leading-6'>
          <p>
            It compiles Python source into bytecode and executes it with a small
            virtual machine. This playground runs the compiler and VM in the
            browser through WebAssembly.
          </p>

          <p>
            Use the editor to try short Python programs, then run them to see
            captured output and the bytecode instructions emitted by the
            compiler.
          </p>

          <a
            href='https://github.com/terror/taipan'
            target='_blank'
            rel='noreferrer'
            className='text-foreground w-fit font-medium underline-offset-4 hover:underline'
          >
            View the project on GitHub
          </a>
        </div>
      </DialogContent>
    </Dialog>
  );
};
