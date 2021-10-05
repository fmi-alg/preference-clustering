SHELL = /bin/bash
CARGO_TARGET_DIR ?= ../../pref-polys/target
HS_GEN_TARGET_DIR = ../../hs_gen/build
SOLVE_HS_TARGET_DIR = ../../solve_hs
GRAPH != grep graph config.yml | cut -d' ' -f2
SEED != grep seed config.yml | cut -d' ' -f2

.PRECIOUS: %.yml %.space %.init_sets %.sets %.lp %.ilpsol %.lpsol %.greedysol %.naivegreedysol
.PHONY: all clean

results.txt: all
	@echo "Naive Greedy Solution: " > results.txt
	grep "solution:" {outer,exact,inner}.naivegreedysol >> results.txt
	@echo "" >> results.txt
	@echo "LP Relaxiation Solution: " >> results.txt
	grep Values {outer,exact,inner}.lpsol >> results.txt
	@echo "" >> results.txt
	@echo "Greedy (LP Rounding) Solution: " >> results.txt
	grep Best {outer,exact,inner}.greedysol >> results.txt
	@echo "" >> results.txt
	@echo "ILP Solution: " >> results.txt
	grep Values {outer,exact,inner}.ilpsol >> results.txt

all: spaces.containment_check inner.greedysol outer.greedysol exact.greedysol inner.ilpsol outer.ilpsol exact.ilpsol inner.naivegreedysol outer.naivegreedysol exact.naivegreedysol
times: inner.times outer.times exact.times

inner.space outer.space paths.yml: config.yml 
	@echo "started at $(shell date)" >> approximation.log
	/usr/bin/time -f "approximation time: %E" $(CARGO_TARGET_DIR)/release/random_approx_instances --config-file config.yml >> approximation.log 2>&1 

exact.space: paths.yml config.yml
	@echo "started at $(shell date)" >> $@.log
	/usr/bin/time -f "$@ time: %E" $(CARGO_TARGET_DIR)/release/exact_preference_areas $(GRAPH) -m 3 -f paths.yml -s $(SEED) -o $@  >> $@.log 2>&1 

spaces.containment_check: outer.space exact.space inner.space
	@echo "started at $(shell date)" >> $@
	/usr/bin/time -f "$@ time: %E" "${HS_GEN_TARGET_DIR}"/contained  $^  >> $@  2>&1

%.init_sets: %.space
	@echo "started at $(shell date)" >> $@.log
	/usr/bin/time -f "$@ time: %E" "${HS_GEN_TARGET_DIR}"/arrang $< >> $@.log 2>&1
	mv Sets.out $@
	mv Sets.pts $@.pts

%.sets: %.init_sets
	@echo "started at $(shell date)" >> $@.log
	/usr/bin/time -f "$@ time: %E" "${HS_GEN_TARGET_DIR}"/SetMinimizer $< >> $@.log 2>&1
	mv Sets.surv $@

%.lp: %.sets
	@echo "started at $(shell date)" >> $@.log
	/usr/bin/time -f "$@ time: %E" "${SOLVE_HS_TARGET_DIR}"/print-ilp < $< > $@ 2>> $@.log

%.ilpsol: %.lp
	@echo "started at $(shell date)" >> $@.log
	/usr/bin/time -f "$@ time: %E" glpsol -o $@ --lp --tmlim 3600 $< >> $@.log 2>&1

%.lpsol: %.lp
	@echo "started at $(shell date)" >> $@.log
	/usr/bin/time -f "$@ time: %E" glpsol -o $@ --lp $< --nomip >> $@.log 2>&1

%.greedysol: %.sets %.lpsol
	@echo "started at $(shell date)" >> $@.log
	/usr/bin/time -f "$@ time: %E" "${SOLVE_HS_TARGET_DIR}"/greedy-solver $^ $(SEED) > $@ 2>> $@.log

%.naivegreedysol: %.sets
	@echo "started at $(shell date)" >> $@.log
	/usr/bin/time -f "$@ time: %E" "${SOLVE_HS_TARGET_DIR}"/naive-greedy-solver < $^  > $@ 2>> $@.log

exact.times: exact.space.log exact.init_sets.log exact.sets.log exact.lp.log exact.lpsol.log exact.greedysol.log exact.naivegreedysol.log exact.ilpsol.log
	for log in $^; do \
		grep time -h $$log >> $@; \
	done

%.times: approximation.log %.init_sets.log %.sets.log %.lp.log %.lpsol.log %.greedysol.log %.naivegreedysol.log %.ilpsol.log
	for log in $^; do \
		grep time -h $$log >> $@; \
	done



clean:
	rm -f *space *sets *lp *sol *log *pts path.yml *.containment_check


