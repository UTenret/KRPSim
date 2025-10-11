## Parse

-   Parse the file, check for errors
-   Fill our structs with stocks and processes

there can only be one line with optimize
order doesnt matter

processes can take nothing or give nothing

## GA

We can parse and know which resources are renewable and which are non-renewable.

We need random keys, sorted processes directly in genome.
Crossover & mutation for keys & divider.

## Specific problems to solve

-   Processes which consumes resources and give nothing back (manger, recre)
-   Processes with consumes resources and give back resources which we dont need or lead to an infinite loop (separation_oeuf/reunion_oeuf, pomme)
-   Having to keep some stock to keep higher production or have production at all over long term instead of rushing (if maximizing quantity) towards goal
-

### Summary

-   loop
-   dead process
-   inefficient producer

### algo

do tau, it effectively is the same as potentially removing processes as well
if max dur is longest process \* 1.5
last process with high tau could wait forever
tau vector, any process could then be effectively locked out


### Python requir txt
