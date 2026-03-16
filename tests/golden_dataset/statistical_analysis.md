# Statistical Analysis for Researchers

## Overview

Statistical analysis transforms raw data into interpretable evidence. Choosing the right statistical test requires understanding the research design, the measurement level of variables, the distribution of data, and the assumptions underlying each test. Misapplied statistics produce misleading results even from valid data.

## Descriptive Statistics

Descriptive statistics summarize a dataset without making inferences about a population. Measures of central tendency — mean, median, mode — describe the center of a distribution. Measures of dispersion — variance, standard deviation, interquartile range — describe its spread. The mean is sensitive to outliers; the median is robust. Always report both the center and the spread.

## Inferential Statistics

### Hypothesis Testing

Null hypothesis significance testing (NHST) evaluates whether observed data are consistent with a null hypothesis (no effect). The p-value is the probability of observing data at least as extreme as the observed data, given the null. A p-value below the significance threshold (commonly 0.05) is not proof of an effect — it is evidence against the null under frequentist assumptions.

### Effect Sizes

Statistical significance does not imply practical importance. Effect size measures — Cohen's d for mean differences, Pearson's r for correlations, odds ratios for categorical outcomes — quantify the magnitude of effects independent of sample size. Reporting effect sizes alongside p-values is now required by major journals.

### Confidence Intervals

A 95% confidence interval contains the true population parameter in 95% of hypothetical replications of the study. Wide intervals reflect uncertainty; narrow intervals reflect precision. Confidence intervals communicate more information than p-values alone.

## Common Pitfalls

Multiple comparisons inflate the Type I error rate. Bonferroni correction or false discovery rate control addresses this. P-hacking — trying multiple analyses until a significant result appears — is a form of research misconduct. Pre-registering analysis plans before data collection prevents this bias.

## Statistical Software

R and Python (with SciPy, statsmodels, and pingouin) are the dominant open-source platforms for statistical analysis. SPSS and SAS remain common in social and health sciences. Reproducible analysis requires that all steps, from data cleaning through final output, be scripted rather than performed through point-and-click interfaces.
